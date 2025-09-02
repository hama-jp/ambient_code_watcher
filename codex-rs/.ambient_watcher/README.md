# Ambient Watcher プロジェクト設定

このディレクトリには、Ambient Watcherのプロジェクト固有の設定が含まれています。

## 🚀 クイックスタート

```bash
# Ambient Watcherを起動
ambient

# ブラウザも自動で開く
ambient --open
```

## 📁 ファイル構成

```
.ambient_watcher/
├── config.toml      # メイン設定ファイル
├── prompts/         # カスタムプロンプトファイル（オプション）
└── README.md        # このファイル
```

## ⚙️ 設定のカスタマイズ

### レビューの追加

`config.toml`に新しいレビューセクションを追加：

```toml
[[reviews]]
name = "カスタムレビュー"
description = "独自のレビュー観点"
file_patterns = ["src/**/*.rs"]
priority = 300  # 高いほど優先
enabled = true
prompt = """
ここにレビューのプロンプトを記述
{file_path}は自動的にファイルパスに置換されます
"""
```

### 除外パターンの設定

特定のファイルやディレクトリを除外：

```toml
exclude_patterns = [
    "target/**",
    "tests/**",
    "*.generated.rs"
]
```

### レビューの優先順位

- `priority`値が高いレビューから順に実行されます
- デフォルト値は100
- 推奨値：
  - 300+: 最重要（ビルドエラーなど）
  - 200: 重要（構文エラー）
  - 150: 中程度（セキュリティ）
  - 100: 通常（最適化提案）
  - 50以下: 低優先度

## 🎯 ファイルパターン

以下のパターンが使用できます：

- `*.rs` - 拡張子による指定
- `src/**/*.rs` - ディレクトリ配下の全ファイル
- `*` - すべてのファイル
- `!test_*.rs` - 除外パターン（exclude_patternsで使用）

## 💡 Tips

1. **特定のレビューを無効化**: `enabled = false`を設定
2. **プロジェクト全体を無効化**: トップレベルの`enabled = false`
3. **長いプロンプト**: `prompts/`ディレクトリに別ファイルとして保存可能

## 🔧 トラブルシューティング

- 設定が反映されない場合は、Ambient Watcherを再起動してください
- 構文エラーがある場合は、`ambient`コマンド実行時にエラーが表示されます
