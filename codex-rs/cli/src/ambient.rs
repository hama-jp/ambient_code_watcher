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
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use tokio::sync::broadcast;

use crate::ambient_server::{run_server, AmbientEvent};

#[derive(Debug, Parser)]
pub struct AmbientCommand {
    #[clap(skip)]
    pub config_overrides: CliConfigOverrides,
}

pub async fn run_main(cmd: AmbientCommand) -> Result<()> {
    const AMBIENT_CHECK_INTERVAL: Duration = Duration::from_secs(10);

    let cli_overrides = cmd
        .config_overrides
        .parse_overrides()
        .map_err(|e| anyhow::anyhow!(e))?;
    let config = Config::load_with_cli_overrides(cli_overrides, Default::default())?;
    let client = reqwest::Client::new();
    let cwd = std::env::current_dir()?;

    // Create the broadcast channel for communication between the server and the analysis loop
    let (tx, mut rx) = broadcast::channel::<AmbientEvent>(100);

    // Start the web server in a separate task
    let server_tx = tx.clone();
    tokio::spawn(async move {
        run_server(server_tx).await;
    });

    let mut ticker = tokio::time::interval(AMBIENT_CHECK_INTERVAL);

    println!("Ambient agent started. Press Ctrl+C to stop.");
    // The UI address is printed by the server itself.

    loop {
        tokio::select! {
            // Listen for user queries from the web UI
            Ok(event) = rx.recv() => {
                if let AmbientEvent::UserQuery(prompt_text) = event {
                    let _ = tx.send(AmbientEvent::Analysis(format!("\n--- Running one-off analysis for: \"{}\" ---", prompt_text.trim())));
                    if let Err(e) = run_analysis_prompt(prompt_text.trim().to_string(), &config, &client, &tx).await {
                        let _ = tx.send(AmbientEvent::Analysis(format!("Error running analysis: {e}")));
                    }
                    let _ = tx.send(AmbientEvent::Analysis("--- Finished one-off analysis ---".to_string()));
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
                println!("\nShutting down ambient agent...");
                break;
            }
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

    let provider = config
        .model_providers
        .get(&config.model_provider_id)
        .ok_or_else(|| anyhow::anyhow!("Provider not found: {}", config.model_provider_id))?;

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

async fn perform_ambient_check(
    config: &Config,
    client: &reqwest::Client,
    cwd: &Path,
    tx: &broadcast::Sender<AmbientEvent>,
) -> Result<()> {
    let output = Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .current_dir(cwd)
        .output()?;

    if output.stdout.is_empty() {
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();

    if !lines.is_empty() {
        let msg = format!(
            "[{}] Found {} changed file(s).",
            chrono::Local::now().to_rfc2822(),
            lines.len()
        );
        let _ = tx.send(AmbientEvent::Analysis(msg));
    }

    for line in lines {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let file_path = parts[1];
        let _ = tx.send(AmbientEvent::Analysis(format!(
            "--- Analyzing: {file_path} ---"
        )));

        // Task A: Diff-based analysis
        let diff_output = Command::new("git")
            .arg("diff")
            .arg("HEAD")
            .arg("--")
            .arg(file_path)
            .current_dir(cwd)
            .output()?;
        let diff_content = String::from_utf8_lossy(&diff_output.stdout);

        if !diff_content.trim().is_empty() {
            // A.1: Summarize changes
            let _ = tx.send(AmbientEvent::Analysis(
                "\n[1/4] Summary of changes:".to_string(),
            ));
            let prompt1 = format!(
                "You are an ambient AI assistant. Here is the diff for `{file_path}`. Please provide a brief, one-sentence summary of the changes.\n\n---\n\n{diff_content}"
            );
            if let Err(e) = run_analysis_prompt(prompt1, config, client, tx).await {
                let _ = tx.send(AmbientEvent::Analysis(format!("Error: {e}")));
            }

            // A.2: Secret detection
            let _ = tx.send(AmbientEvent::Analysis(
                "\n[2/4] Secret detection:".to_string(),
            ));
            let prompt2 = format!(
                "You are a security scanner. Analyze the following diff for `{file_path}` and report any potential secrets like API keys or credentials. If none are found, say 'No secrets found'.\n\n---\n\n{diff_content}"
            );
            if let Err(e) = run_analysis_prompt(prompt2, config, client, tx).await {
                let _ = tx.send(AmbientEvent::Analysis(format!("Error: {e}")));
            }
        } else {
            let _ = tx.send(AmbientEvent::Analysis(
                "\nSkipping diff analysis (file is new or unstaged).".to_string(),
            ));
        }

        // Task B: Full-file analysis
        let full_path = cwd.join(file_path);
        let content = match fs::read_to_string(&full_path) {
            Ok(c) => c,
            Err(e) => {
                let err_msg = format!("Failed to read file {}: {e}", full_path.display());
                let _ = tx.send(AmbientEvent::Analysis(err_msg));
                continue;
            }
        };

        // B.1: Magic number detection
        let _ = tx.send(AmbientEvent::Analysis(
            "\n[3/4] Magic number detection:".to_string(),
        ));
        let prompt3 = format!(
            "You are a code quality assistant. Analyze the following file `{file_path}` and identify any hard-coded constants (magic numbers). If any are found, suggest defining them as named constants. If none are found, say 'No magic numbers found'.\n\n---\n\n{content}"
        );
        if let Err(e) = run_analysis_prompt(prompt3, config, client, tx).await {
            let _ = tx.send(AmbientEvent::Analysis(format!("Error: {e}")));
        }

        // B.2: Complexity/Duplication detection
        let _ = tx.send(AmbientEvent::Analysis(
            "\n[4/4] Complexity and duplication check:".to_string(),
        ));
        let prompt4 = format!(
            "You are a code quality assistant. Analyze the following file `{file_path}` for overly complex functions or duplicated code blocks. If any are found, suggest refactoring or adding comments for clarity. If none are found, say 'No complexity or duplication issues found'.\n\n---\n\n{content}"
        );
        if let Err(e) = run_analysis_prompt(prompt4, config, client, tx).await {
            let _ = tx.send(AmbientEvent::Analysis(format!("Error: {e}")));
        }
        let _ = tx.send(AmbientEvent::Analysis(format!(
            "--- Finished analyzing: {file_path} ---\n"
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
