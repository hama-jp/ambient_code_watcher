# Ambient Watcher

リアルタイムコードレビュー支援ツール - [Codex](https://github.com/anthropics/codex)をベースに開発

> **Note**: オリジナルのCodex READMEは[ORIGINAL_README.md](ORIGINAL_README.md)をご覧ください。

## 概要

Ambient Watcherは、コード変更を自動的に検出し、ローカルLLM（Ollama）を使用してリアルタイムでコードレビューを行うツールです。Web UIを通じて、レビュー結果の確認や対話的な質問が可能です。

## 特徴

- 🔍 **自動コードレビュー** - Git変更を検出して自動的にレビュー
- 💬 **対話的な質問** - Web UIから特定の疑問を直接質問
- ⚙️ **柔軟な設定** - プロジェクトごとにレビュー観点をカスタマイズ
- 🌐 **Web UI** - ブラウザベースの使いやすいインターフェース
- 🔒 **プライバシー重視** - すべての処理はローカルで完結

## インストール

### 前提条件

- Rust (1.70以降)
- [Ollama](https://ollama.ai/) 
- Git

### セットアップ

```bash
# リポジトリをクローン
git clone https://github.com/yourusername/codex.git
cd codex/codex-rs

# 自動インストール（推奨）
./install.sh

# または手動ビルド
cargo build --release --bin ambient-watcher

# Ollamaモデルをダウンロード（推奨）
ollama pull gpt-oss:20b
```

### インストールスクリプト

`install.sh`を使用すると以下が自動的に設定されます：

- 実行ファイルを`~/.local/bin/`にインストール
- `ambient`コマンドをグローバルに利用可能に
- デフォルト設定ファイルの作成
- PATH設定の案内

```bash
# インストール
./install.sh

# アンインストール
./uninstall.sh
```

## 使い方

### 基本的な使用方法

```bash
# プロジェクトで初回設定
ambient init

# Ambient Watcherを起動
ambient

# ブラウザも自動で開く
ambient --open
```

### Web UI

起動後、`http://localhost:38080` でWeb UIにアクセスできます。

- リアルタイムでレビュー結果を表示
- 下部の入力欄から質問可能
- Markdown形式の整形された出力

## 設定

### プロジェクト設定 (`.ambient_watcher/config.toml`)

```toml
[[reviews]]
name = "カスタムレビュー"
description = "プロジェクト固有のレビュー"
file_patterns = ["src/**/*.rs"]
priority = 200
enabled = true
prompt = """
以下の観点でレビューしてください：
1. エラーハンドリング
2. パフォーマンス
3. セキュリティ
"""
```

### グローバル設定 (`~/.codex/ambient.toml`)

```toml
# チェック間隔（秒）
check_interval_secs = 60

# WebUIのポート
port = 38080
```

## プロジェクト構成

```
codex-rs/
├── cli/src/
│   ├── ambient.rs              # メインロジック
│   ├── ambient_server.rs        # WebSocketサーバー
│   ├── ambient_config.rs        # グローバル設定
│   ├── ambient_project_config.rs # プロジェクト設定
│   └── ambient_ui/              # Web UIファイル
├── ambient                      # 起動スクリプト
└── ambient-init                 # 初期化スクリプト
```

## カスタマイズ

### レビュー観点の追加

`.ambient_watcher/config.toml`を編集して、独自のレビュー観点を追加できます：

- `file_patterns`: 対象ファイルのパターン
- `priority`: 実行優先度（高い値が優先）
- `prompt`: レビュー時のプロンプト

### 除外パターン

特定のファイルやディレクトリを除外：

```toml
exclude_patterns = [
    "target/**",
    "*.generated.rs",
    "tests/**"
]
```

## トラブルシューティング

### ポートが使用中の場合
Ambient Watcherは自動的に次のポート（38081, 38082...）を試します。

### Ollamaが動作しない場合
```bash
# Ollamaの状態確認
ollama list

# サービスの再起動
ollama serve
```

## セキュリティとプライバシー

- すべての処理はローカルで実行
- 外部サーバーへのコード送信なし
- XSS対策としてDOMPurifyを使用
- プロジェクト設定は`.ambient_watcher/`に保存

## ライセンス

このプロジェクトは[Codex](https://github.com/anthropics/codex)をベースに開発されています。
オリジナルのCodexプロジェクトのライセンス条項に従います。

## 謝辞

- [Anthropic Codex](https://github.com/anthropics/codex) - 本プロジェクトのベース
- [Ollama](https://ollama.ai/) - ローカルLLM実行環境
- すべてのコントリビューター

## コントリビューション

Issue報告やPull Requestを歓迎します。大きな変更の場合は、事前にIssueで議論をお願いします。

---

*Ambient Watcher - Making code review ambient and effortless*