use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// プロジェクトごとのAmbient Code Watcher設定
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectConfig {
    /// Ollama設定
    #[serde(default)]
    pub ollama: OllamaConfig,
    
    /// ファイル変更の検出間隔（秒）
    #[serde(default = "default_check_interval")]
    pub check_interval_secs: u64,
    
    /// Web UIのポート番号
    #[serde(default = "default_port")]
    pub port: u16,
    
    /// レビューを有効にするかどうか
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    
    /// 除外パターン
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
    
    /// カスタムプロンプト
    #[serde(default)]
    pub custom_prompts: Vec<CustomPrompt>,
    
    /// 分析を有効にする拡張子のリスト
    #[serde(default = "default_file_extensions")]
    pub file_extensions: Vec<String>,
    
    /// レビュー設定
    #[serde(default)]
    pub reviews: Vec<ReviewConfig>,
}

/// Ollama設定
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OllamaConfig {
    /// OllamaのベースURL
    #[serde(default = "default_ollama_base_url")]
    pub base_url: String,
    
    /// 使用するモデル名
    #[serde(default = "default_ollama_model")]
    pub model: String,
}

/// 個別のレビュー設定
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReviewConfig {
    /// レビューの名前
    pub name: String,
    
    /// レビューの説明
    #[serde(default)]
    pub description: String,
    
    /// このレビューを適用するファイルパターン
    pub file_patterns: Vec<String>,
    
    /// レビューのプロンプト
    pub prompt: String,
    
    /// 優先度（高い順に実行）
    #[serde(default = "default_priority")]
    pub priority: u32,
    
    /// このレビューを有効にするか
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

/// カスタムプロンプト
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomPrompt {
    /// プロンプトID
    pub id: String,
    
    /// プロンプトの内容
    pub content: String,
}

fn default_enabled() -> bool {
    true
}

fn default_check_interval() -> u64 {
    60 // デフォルト60秒
}

fn default_port() -> u16 {
    38080
}

fn default_file_extensions() -> Vec<String> {
    vec![
        "rs".to_string(),
        "toml".to_string(),
        "js".to_string(),
        "ts".to_string(),
        "jsx".to_string(),
        "tsx".to_string(),
        "py".to_string(),
        "go".to_string(),
        "java".to_string(),
        "cpp".to_string(),
        "c".to_string(),
        "h".to_string(),
        "hpp".to_string(),
        "cs".to_string(),
        "rb".to_string(),
        "php".to_string(),
        "swift".to_string(),
        "kt".to_string(),
        "scala".to_string(),
        "sh".to_string(),
        "bash".to_string(),
        "zsh".to_string(),
        "fish".to_string(),
        "yml".to_string(),
        "yaml".to_string(),
        "json".to_string(),
        "xml".to_string(),
        "html".to_string(),
        "css".to_string(),
        "scss".to_string(),
        "sass".to_string(),
        "less".to_string(),
        "sql".to_string(),
        "md".to_string(),
        "mdx".to_string(),
    ]
}

fn default_priority() -> u32 {
    100
}

fn default_ollama_base_url() -> String {
    "http://localhost:11434/v1".to_string()
}

fn default_ollama_model() -> String {
    "gpt-oss:20b".to_string()
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: default_ollama_base_url(),
            model: default_ollama_model(),
        }
    }
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            ollama: OllamaConfig::default(),
            check_interval_secs: default_check_interval(),
            port: default_port(),
            enabled: true,
            exclude_patterns: vec![
                "target/**".to_string(),
                "node_modules/**".to_string(),
                ".git/**".to_string(),
                "*.min.js".to_string(),
            ],
            custom_prompts: vec![],
            file_extensions: default_file_extensions(),
            reviews: vec![
                ReviewConfig {
                    name: "構文エラー・型エラーチェック".to_string(),
                    description: "コードの構文エラーと型の不一致を検出".to_string(),
                    file_patterns: vec!["*.rs".to_string(), "*.ts".to_string(), "*.js".to_string()],
                    prompt: "以下のコードを分析して、構文エラーや型エラーの可能性を日本語で報告してください：\n1. 未定義変数、括弧の不一致、セミコロン忘れ\n2. 型の不一致\n3. エラー箇所は`{file_path}:行番号`形式で".to_string(),
                    priority: 200,
                    enabled: true,
                },
                ReviewConfig {
                    name: "セキュリティリスク検出".to_string(),
                    description: "セキュリティ脆弱性とハードコードされた秘密情報を検出".to_string(),
                    file_patterns: vec!["*".to_string()],
                    prompt: "以下のコードのセキュリティリスクを日本語で報告してください：\n1. ハードコードされたAPIキー、パスワード、トークン\n2. SQLインジェクション、XSSの脆弱性\n3. 安全でない入力検証".to_string(),
                    priority: 150,
                    enabled: true,
                },
                ReviewConfig {
                    name: "パフォーマンス最適化".to_string(),
                    description: "パフォーマンス問題と最適化の機会を検出".to_string(),
                    file_patterns: vec!["*.rs".to_string(), "*.go".to_string(), "*.cpp".to_string()],
                    prompt: "以下のコードのパフォーマンス問題を日本語で分析してください：\n1. O(n²)以上の計算量\n2. 不要なループやメモリリーク\n3. より効率的な実装方法の提案".to_string(),
                    priority: 100,
                    enabled: true,
                },
            ],
        }
    }
}

impl ProjectConfig {
    /// プロジェクト設定を読み込む
    pub fn load_from_project(project_path: &Path) -> Result<Self> {
        let config_dir = project_path.join(".ambient");
        let config_file = config_dir.join("config.toml");
        
        if config_file.exists() {
            let content = fs::read_to_string(&config_file)?;
            let config: Self = toml::from_str(&content)?;
            Ok(config)
        } else {
            // デフォルト設定を返す
            Ok(Self::default())
        }
    }
    
    /// プロジェクト設定を保存する
    pub fn save_to_project(&self, project_path: &Path) -> Result<()> {
        let config_dir = project_path.join(".ambient");
        fs::create_dir_all(&config_dir)?;
        
        let config_file = config_dir.join("config.toml");
        
        // TOMLの順序を制御するために手動でフォーマット
        let mut content = String::new();
        
        // Ollama設定を最初に配置
        content.push_str("# Ollama設定\n");
        content.push_str("[ollama]\n");
        content.push_str(&format!("base_url = \"{}\"\n", self.ollama.base_url));
        content.push_str(&format!("model = \"{}\"\n", self.ollama.model));
        content.push_str("\n");
        
        // 基本設定
        content.push_str("# 基本設定\n");
        content.push_str(&format!("check_interval_secs = {}\n", self.check_interval_secs));
        content.push_str(&format!("port = {}\n", self.port));
        content.push_str(&format!("enabled = {}\n", self.enabled));
        content.push_str("\n");
        
        // 除外パターン
        content.push_str("# 除外パターン\n");
        content.push_str("exclude_patterns = [\n");
        for pattern in &self.exclude_patterns {
            content.push_str(&format!("    \"{}\",\n", pattern));
        }
        content.push_str("]\n");
        content.push_str("custom_prompts = []\n");
        
        // ファイル拡張子
        content.push_str("file_extensions = [\n");
        for ext in &self.file_extensions {
            content.push_str(&format!("    \"{}\",\n", ext));
        }
        content.push_str("]\n");
        content.push_str("\n");
        
        // レビュー設定
        for review in &self.reviews {
            content.push_str("[[reviews]]\n");
            content.push_str(&format!("name = \"{}\"\n", review.name));
            content.push_str(&format!("description = \"{}\"\n", review.description));
            content.push_str("file_patterns = [\n");
            for pattern in &review.file_patterns {
                content.push_str(&format!("    \"{}\",\n", pattern));
            }
            content.push_str("]\n");
            content.push_str(&format!("prompt = \"\"\"\n{}\"\"\"\n", review.prompt));
            content.push_str(&format!("priority = {}\n", review.priority));
            content.push_str(&format!("enabled = {}\n", review.enabled));
            content.push_str("\n");
        }
        
        fs::write(&config_file, content)?;
        
        Ok(())
    }
    
    /// サンプル設定ファイルを生成
    pub fn create_sample(project_path: &Path) -> Result<()> {
        let config = Self::default();
        config.save_to_project(project_path)?;
        
        // READMEも作成
        let config_dir = project_path.join(".ambient");
        let readme_path = config_dir.join("README.md");
        let readme_content = r#"# Ambient Code Watcher プロジェクト設定

このディレクトリには、Ambient Code Watcherのプロジェクト固有の設定が含まれています。

## ファイル構成

- `config.toml` - メイン設定ファイル
- `prompts/` - カスタムプロンプトファイル（オプション）

## 設定のカスタマイズ

`config.toml`を編集して、レビューの内容や観点をカスタマイズできます。

### 基本設定

```toml
# ファイル変更の検出間隔（秒）
check_interval_secs = 60

# Web UIのポート番号
port = 38080

# Ollama設定
[ollama]
base_url = "http://localhost:11434/v1"
model = "gpt-oss:20b"
```

### レビュー設定の例

```toml
[[reviews]]
name = "カスタムレビュー"
description = "プロジェクト固有のレビュー"
file_patterns = ["src/**/*.rs"]
prompt = "このコードをプロジェクトの規約に従ってレビューしてください"
priority = 300
enabled = true
```

### 除外パターンの設定

```toml
exclude_patterns = [
    "target/**",
    "tests/**",
    "*.generated.rs"
]
```

## プロンプトのカスタマイズ

長いプロンプトは別ファイルに保存して参照することもできます：

1. `prompts/`ディレクトリにファイルを作成
2. `custom_prompts`で参照

```toml
[[custom_prompts]]
id = "architecture_review"
content = """
このコードのアーキテクチャをレビューしてください：
1. SOLID原則の遵守
2. 依存関係の適切性
3. モジュール間の結合度
"""
```
"#;
        fs::write(&readme_path, readme_content)?;
        
        Ok(())
    }
    
    /// ファイルパスに適用するレビューを取得
    pub fn get_reviews_for_file(&self, file_path: &str) -> Vec<&ReviewConfig> {
        let mut reviews: Vec<&ReviewConfig> = self.reviews
            .iter()
            .filter(|r| r.enabled && self.matches_patterns(file_path, &r.file_patterns))
            .collect();
        
        // 優先度順にソート（高い順）
        reviews.sort_by(|a, b| b.priority.cmp(&a.priority));
        reviews
    }
    
    /// ファイルパスがパターンにマッチするか
    fn matches_patterns(&self, file_path: &str, patterns: &[String]) -> bool {
        for pattern in patterns {
            if pattern == "*" {
                return true;
            }
            
            // 簡単なglob実装
            if pattern.starts_with("*.") {
                let ext = pattern.trim_start_matches("*.");
                if file_path.ends_with(&format!(".{}", ext)) {
                    return true;
                }
            }
            
            if pattern.ends_with("/**") {
                let prefix = pattern.trim_end_matches("/**");
                if file_path.starts_with(prefix) {
                    return true;
                }
            }
            
            if glob::Pattern::new(pattern).ok().map_or(false, |p| p.matches(file_path)) {
                return true;
            }
        }
        false
    }
    
    /// ファイルが除外パターンにマッチするか
    pub fn is_excluded(&self, file_path: &str) -> bool {
        self.matches_patterns(file_path, &self.exclude_patterns)
    }
}