use anyhow::Result;
use clap::Parser;
use codex_common::CliConfigOverrides;
use codex_core::chat_completions::stream_chat_completions;
use codex_core::client_common::Prompt;
use codex_core::client_common::ResponseEvent;
use codex_core::config::Config;
use codex_core::model_family;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use futures::StreamExt;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use tokio::sync::broadcast;

use crate::ambient_server::{run_server, AmbientEvent};
use crate::ambient_config::AmbientConfig;
use crate::ambient_project_config::ProjectConfig;

#[derive(Debug, Parser)]
pub struct AmbientCommand {
    #[clap(skip)]
    pub config_overrides: CliConfigOverrides,
}

pub async fn run_main(cmd: AmbientCommand) -> Result<()> {
    // 設定ファイルを読み込む
    let ambient_config = AmbientConfig::load()?;
    let check_interval = Duration::from_secs(ambient_config.check_interval_secs);
    
    println!("検出間隔: {}秒", ambient_config.check_interval_secs);

    let mut cli_overrides = cmd
        .config_overrides
        .parse_overrides()
        .map_err(|e| anyhow::anyhow!(e))?;
    
    // Force OSS provider for ambient mode
    // Note: We need to use toml::Value here, not serde_json::Value
    use toml::Value;
    cli_overrides.push(("model_provider_id".to_string(), Value::String("oss".to_string())));
    cli_overrides.push(("model".to_string(), Value::String("gpt-oss:20b".to_string())));
    
    let mut config = Config::load_with_cli_overrides(cli_overrides, Default::default())?;
    
    // Force set the provider ID after loading
    config.model_provider_id = "oss".to_string();
    
    // Also update the model_provider field to match the OSS provider
    if let Some(oss_provider) = config.model_providers.get("oss") {
        config.model_provider = oss_provider.clone();
    }
    
    let client = reqwest::Client::new();
    let cwd = std::env::current_dir()?;

    // Create the broadcast channel for communication between the server and the analysis loop
    let (tx, mut rx) = broadcast::channel::<AmbientEvent>(100);

    // Start the web server in a separate task
    let server_tx = tx.clone();
    let server_port = ambient_config.port;
    tokio::spawn(async move {
        run_server(server_tx, server_port).await;
    });

    let mut ticker = tokio::time::interval(check_interval);

    println!("Ambient Watcherが起動しました。終了するにはCtrl+Cを押してください。");
    // The UI address is printed by the server itself.

    loop {
        tokio::select! {
            // Listen for user queries from the web UI
            Ok(event) = rx.recv() => {
                if let AmbientEvent::UserQuery(prompt_text) = event {
                    // 質問への回答用の関数を呼び出す
                    if let Err(e) = run_query_response(prompt_text.trim().to_string(), &config, &client, &tx).await {
                        let _ = tx.send(AmbientEvent::QueryResponse(format!("エラー: {e}")));
                    }
                }
            }

            // Perform ambient check on a timer
            _ = ticker.tick() => {
                if let Err(e) = perform_ambient_check(&config, &client, &cwd, &tx).await {
                    let err_msg = format!("[{}] Error: {}", chrono::Local::now().to_rfc2822(), e);
                    let _ = tx.send(AmbientEvent::Analysis(err_msg));
                }
            }

            // Handle Ctrl-C for graceful shutdown
            _ = tokio::signal::ctrl_c() => {
                println!("\nAmbient Watcherを終了します...");
                break;
            }
        }
    }
    Ok(())
}

// 質問への回答用関数
async fn run_query_response(
    prompt_text: String,
    config: &Config,
    client: &reqwest::Client,
    tx: &broadcast::Sender<AmbientEvent>,
) -> Result<()> {
    let model_family = model_family::find_family_for_model(&config.model)
        .ok_or_else(|| anyhow::anyhow!("Model family not found for: {}", config.model))?;

    let provider = config
        .model_providers
        .get("oss")
        .ok_or_else(|| anyhow::anyhow!("OSS provider not found"))?;

    let user_message = ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText { text: prompt_text }],
    };

    let prompt = Prompt {
        input: vec![user_message],
        store: false,
        tools: vec![],
        base_instructions_override: None,
    };

    let stream_result = stream_chat_completions(&prompt, &model_family, client, provider).await;

    match stream_result {
        Ok(mut stream) => {
            let mut full_response = String::new();
            while let Some(event) = stream.next().await {
                match event {
                    Ok(ResponseEvent::OutputTextDelta(delta)) => {
                        full_response.push_str(&delta);
                    }
                    Ok(ResponseEvent::Completed { .. }) => {
                        break;
                    }
                    Err(e) => {
                        let err_msg = format!("Error processing stream: {e:?}");
                        let _ = tx.send(AmbientEvent::QueryResponse(err_msg.clone()));
                        return Err(anyhow::anyhow!(err_msg));
                    }
                    _ => {}
                }
            }
            // QueryResponseとして送信
            let _ = tx.send(AmbientEvent::QueryResponse(full_response));
        }
        Err(e) => {
            let err_msg = format!("Failed to get AI insight: {e}");
            let _ = tx.send(AmbientEvent::QueryResponse(err_msg.clone()));
            return Err(anyhow::anyhow!(err_msg));
        }
    }
    Ok(())
}

async fn run_analysis_prompt(
    prompt_text: String,
    config: &Config,
    client: &reqwest::Client,
    tx: &broadcast::Sender<AmbientEvent>,
) -> Result<()> {
    let model_family = model_family::find_family_for_model(&config.model)
        .ok_or_else(|| anyhow::anyhow!("Model family not found for: {}", config.model))?;

    // Always use OSS provider for ambient mode
    let provider = config
        .model_providers
        .get("oss")
        .ok_or_else(|| anyhow::anyhow!("OSS provider not found"))?;

    let user_message = ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText { text: prompt_text }],
    };

    let prompt = Prompt {
        input: vec![user_message],
        store: false,
        tools: vec![],
        base_instructions_override: None,
    };

    let stream_result = stream_chat_completions(&prompt, &model_family, client, provider).await;

    match stream_result {
        Ok(mut stream) => {
            let mut full_response = String::new();
            while let Some(event) = stream.next().await {
                match event {
                    Ok(ResponseEvent::OutputTextDelta(delta)) => {
                        full_response.push_str(&delta);
                    }
                    Ok(ResponseEvent::Completed { .. }) => {
                        break;
                    }
                    Err(e) => {
                        let err_msg = format!("Error processing stream: {e:?}");
                        let _ = tx.send(AmbientEvent::Analysis(err_msg.clone()));
                        return Err(anyhow::anyhow!(err_msg));
                    }
                    _ => {}
                }
            }
            // Send the full response at once.
            let _ = tx.send(AmbientEvent::Analysis(full_response));
        }
        Err(e) => {
            let err_msg = format!("Failed to get AI insight: {e}");
            let _ = tx.send(AmbientEvent::Analysis(err_msg.clone()));
            return Err(anyhow::anyhow!(err_msg));
        }
    }
    Ok(())
}

// ヘルパー関数: Gitコマンドの実行と結果チェック
fn run_git_command(args: &[&str], cwd: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Git command failed: {}", stderr));
    }
    
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

// ヘルパー関数: 分析プロンプトの実行
async fn analyze_with_prompt(
    title: &str,
    prompt: String,
    config: &Config,
    client: &reqwest::Client,
    tx: &broadcast::Sender<AmbientEvent>,
) {
    let _ = tx.send(AmbientEvent::Analysis(format!("\n{}", title)));
    if let Err(e) = run_analysis_prompt(prompt, config, client, tx).await {
        let _ = tx.send(AmbientEvent::Analysis(format!("Error: {e}")));
    }
}

async fn perform_ambient_check(
    config: &Config,
    client: &reqwest::Client,
    cwd: &Path,
    tx: &broadcast::Sender<AmbientEvent>,
) -> Result<()> {
    // プロジェクト設定を読み込み
    let project_config = ProjectConfig::load_from_project(cwd).unwrap_or_default();
    
    if !project_config.enabled {
        return Ok(());
    }
    // Git statusを一度だけ実行
    let status_output = run_git_command(&["status", "--porcelain"], cwd)?;
    
    if status_output.trim().is_empty() {
        return Ok(());
    }

    let lines: Vec<&str> = status_output.trim().lines().collect();

    if !lines.is_empty() {
        let msg = format!(
            "[{}] {}個の変更されたファイルが見つかりました。",
            chrono::Local::now().to_rfc2822(),
            lines.len()
        );
        let _ = tx.send(AmbientEvent::Analysis(msg));
    }
    
    // Git rootを一度だけ取得
    let git_root = run_git_command(&["rev-parse", "--show-toplevel"], cwd)?
        .trim()
        .to_string();
    
    // 変更されたファイルを収集
    let mut changed_files = Vec::new();
    for line in lines {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            changed_files.push(parts[1].to_string());
        }
    }
    
    // すべてのdiffを一括で取得
    let mut all_diffs = HashMap::new();
    for file_path in &changed_files {
        if let Ok(diff) = run_git_command(&["diff", "HEAD", "--", file_path], cwd) {
            if !diff.trim().is_empty() {
                all_diffs.insert(file_path.clone(), diff);
            }
        }
    }

    // 各ファイルを分析
    for file_path in changed_files {
        let file_path_str = file_path.as_str();
        
        // 除外パターンをチェック
        if project_config.is_excluded(file_path_str) {
            let _ = tx.send(AmbientEvent::Analysis(format!(
                "[スキップ] {} は除外パターンに一致",
                file_path_str
            )));
            continue;
        }
        let _ = tx.send(AmbientEvent::Analysis(format!(
            "--- 分析中: {file_path_str} ---"
        )));

        // プロジェクト設定に基づいたレビューを実行
        let reviews = project_config.get_reviews_for_file(file_path_str);
        
        if reviews.is_empty() {
            // デフォルトのレビューを実行
            if let Some(diff_content) = all_diffs.get(&file_path) {
                // 構文エラーと型エラーのチェック
                let prompt1 = format!(
                    "あなたはコードレビューアシスタントです。`{file_path_str}`のdiffを分析して、以下を日本語で報告してください：\n\n1. 構文エラーの可能性がある箇所（未定義変数、括弧の不一致、セミコロン忘れなど）\n2. 型の不一致の可能性\n3. エラーがある場合は`{file_path_str}:行番号`の形式でリンクを提供\n\nエラーがない場合は『構文エラーは見つかりませんでした』と答えてください。\n\n---\n\n{diff_content}"
                );
                analyze_with_prompt(
                    "[1/3] 構文エラー・型エラーのチェック:",
                    prompt1,
                    config,
                    client,
                    tx,
                ).await;

                // セキュリティリスクの検出
                let prompt2 = format!(
                    "あなたはセキュリティエキスパートです。`{file_path_str}`のdiffを分析して、以下のセキュリティリスクを日本語で報告してください：\n\n1. ハードコードされたAPIキー、パスワード、トークン\n2. SQLインジェクション、XSSの脆弱性\n3. 安全でない入力検証\n4. エラー箇所は`{file_path_str}:行番号`形式で\n\nリスクがない場合は『セキュリティリスクは見つかりませんでした』と答えてください。\n\n---\n\n{diff_content}"
                );
                analyze_with_prompt(
                    "[2/3] セキュリティリスクの検出:",
                    prompt2,
                    config,
                    client,
                    tx,
                ).await;
            }
        } else {
            // カスタムレビューを実行
            let review_count = reviews.len();
            let mut review_index = 1;
            
            for review in reviews {
                let content = if let Some(diff_content) = all_diffs.get(&file_path) {
                    format!("{}

---

{}", review.prompt.replace("{file_path}", file_path_str), diff_content)
                } else {
                    let full_path = std::path::Path::new(&git_root).join(&file_path);
                    if let Ok(file_content) = fs::read_to_string(&full_path) {
                        format!("{}

---

{}", review.prompt.replace("{file_path}", file_path_str), file_content)
                    } else {
                        continue;
                    }
                };
                
                analyze_with_prompt(
                    &format!("[{}/{}] {}: {}", review_index, review_count, review.name, review.description),
                    content,
                    config,
                    client,
                    tx,
                ).await;
                
                review_index += 1;
            }
        }

        let _ = tx.send(AmbientEvent::Analysis(format!(
            "--- 分析完了: {file_path_str} ---\n"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_core::ModelProviderInfo;
    use codex_core::BUILT_IN_OSS_MODEL_PROVIDER_ID;
    use codex_core::WireApi;
    use codex_core::config_types::History;
    use codex_core::config_types::ShellEnvironmentPolicy;
    use codex_core::config_types::Tui;
    use codex_core::config_types::UriBasedFileOpener;
    use codex_core::model_family::find_family_for_model;
    use codex_core::protocol::AskForApproval;
    use codex_core::protocol::SandboxPolicy;
    use codex_protocol::mcp_protocol::AuthMode;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn setup_test_env() -> (Config, MockServer, tempfile::TempDir) {
        let server = MockServer::start().await;
        let dir = tempdir().unwrap();
        std::process::Command::new("git")
            .arg("init")
            .current_dir(dir.path())
            .output()
            .unwrap();

        let model = "gpt-3.5-turbo".to_string();
        let model_family = find_family_for_model(&model).unwrap();
        let provider_id = BUILT_IN_OSS_MODEL_PROVIDER_ID.to_string();

        let provider_info = ModelProviderInfo {
            name: "test-provider".to_string(),
            base_url: Some(server.uri()),
            env_key: None,
            env_key_instructions: None,
            wire_api: WireApi::Chat,
            query_params: None,
            http_headers: None,
            env_http_headers: None,
            request_max_retries: Some(1),
            stream_max_retries: Some(1),
            stream_idle_timeout_ms: Some(1000),
            requires_openai_auth: false,
        };

        let config = Config {
            model: model.clone(),
            model_family,
            model_provider_id: provider_id.clone(),
            // This is deprecated, but required for now.
            model_provider: provider_info.clone(),
            model_providers: HashMap::from([(provider_id, provider_info)]),
            model_context_window: None,
            model_max_output_tokens: None,
            approval_policy: AskForApproval::OnRequest,
            sandbox_policy: SandboxPolicy::ReadOnly,
            shell_environment_policy: ShellEnvironmentPolicy::default(),
            hide_agent_reasoning: false,
            show_raw_agent_reasoning: false,
            disable_response_storage: false,
            user_instructions: None,
            base_instructions: None,
            notify: None,
            cwd: PathBuf::new(),
            mcp_servers: HashMap::new(),
            project_doc_max_bytes: 0,
            codex_home: PathBuf::new(),
            history: History::default(),
            file_opener: UriBasedFileOpener::VsCode,
            tui: Tui::default(),
            codex_linux_sandbox_exe: None,
            model_reasoning_effort: Default::default(),
            model_reasoning_summary: Default::default(),
            model_verbosity: None,
            chatgpt_base_url: "".to_string(),
            experimental_resume: None,
            include_plan_tool: false,
            include_apply_patch_tool: false,
            tools_web_search_request: false,
            responses_originator_header: "".to_string(),
            preferred_auth_method: AuthMode::ChatGPT,
            use_experimental_streamable_shell_tool: false,
            include_view_image_tool: false,
            disable_paste_burst: false,
        };

        (config, server, dir)
    }

    #[tokio::test]
    async fn test_ambient_check_happy_path() {
        let (config, server, dir) = setup_test_env().await;
        let client = reqwest::Client::new();
        let (tx, _rx) = broadcast::channel::<AmbientEvent>(1);

        // Create a dummy file change
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "hello").unwrap();
        std::process::Command::new("git")
            .arg("add")
            .arg("test.txt")
            .current_dir(dir.path())
            .output()
            .unwrap();

        // Mock the AI server response
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                "data: {\"choices\": [{\"delta\": {\"content\": \"summary\"}}]}\n\ndata: [DONE]\n\n",
            ))
            .mount(&server)
            .await;

        let result = perform_ambient_check(&config, &client, dir.path(), &tx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_ambient_check_api_error() {
        let (config, server, dir) = setup_test_env().await;
        let client = reqwest::Client::new();
        let (tx, _rx) = broadcast::channel::<AmbientEvent>(1);

        // Create a dummy file change
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "hello").unwrap();
        std::process::Command::new("git")
            .arg("add")
            .arg("test.txt")
            .current_dir(dir.path())
            .output()
            .unwrap();

        // Mock the AI server to return an error
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let result = perform_ambient_check(&config, &client, dir.path(), &tx).await;
        // The new logic continues on error, so the overall result should be Ok.
        // The errors are printed to stderr, but the test doesn't capture that.
        // We are asserting that the function doesn't panic and completes.
        assert!(result.is_ok());
    }
}
