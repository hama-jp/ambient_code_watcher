use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// プロジェクトごとのAmbient Code Watcher設定
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectConfig {
    /// レビュー設定
    #[serde(default)]
    pub reviews: Vec<ReviewConfig>,
    
    /// 除外パターン
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
    
    /// カスタムプロンプト
    #[serde(default)]
    pub custom_prompts: Vec<CustomPrompt>,
    
    /// レビューを有効にするかどうか
    #[serde(default = "default_enabled")]
    pub enabled: bool,
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

fn default_priority() -> u32 {
    100
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
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
            exclude_patterns: vec![
                "target/**".to_string(),
                "node_modules/**".to_string(),
                ".git/**".to_string(),
                "*.min.js".to_string(),
            ],
            custom_prompts: vec![],
            enabled: true,
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
        let content = toml::to_string_pretty(self)?;
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