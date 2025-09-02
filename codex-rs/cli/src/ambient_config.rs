use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AmbientConfig {
    /// ファイル変更の検出間隔（秒）
    #[serde(default = "default_check_interval")]
    pub check_interval_secs: u64,
    
    /// Web UIのポート番号
    #[serde(default = "default_port")]
    pub port: u16,
    
    /// 分析を有効にする拡張子のリスト
    #[serde(default = "default_file_extensions")]
    pub file_extensions: Vec<String>,
}

impl Default for AmbientConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: default_check_interval(),
            port: default_port(),
            file_extensions: default_file_extensions(),
        }
    }
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

impl AmbientConfig {
    /// 設定ファイルを読み込む
    pub fn load() -> anyhow::Result<Self> {
        let config_path = Self::config_path()?;
        
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let config: AmbientConfig = toml::from_str(&content)?;
            println!("設定ファイルを読み込みました: {}", config_path.display());
            Ok(config)
        } else {
            // 設定ファイルが存在しない場合はデフォルト設定を作成
            let config = Self::default();
            config.save()?;
            println!("デフォルト設定ファイルを作成しました: {}", config_path.display());
            Ok(config)
        }
    }
    
    /// 設定ファイルを保存する
    pub fn save(&self) -> anyhow::Result<()> {
        let config_path = Self::config_path()?;
        let content = toml::to_string_pretty(self)?;
        
        // ディレクトリが存在しない場合は作成
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::write(&config_path, content)?;
        Ok(())
    }
    
    /// 設定ファイルのパスを取得
    fn config_path() -> anyhow::Result<PathBuf> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| anyhow::anyhow!("ホームディレクトリが見つかりません"))?;
        
        Ok(PathBuf::from(home)
            .join(".codex")
            .join("ambient.toml"))
    }
}