#!/bin/bash
# Ambient Code Watcher インストールスクリプト

set -e

echo "========================================="
echo "  Ambient Code Watcher インストーラー"
echo "========================================="
echo ""

# カラーコード
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# インストール先ディレクトリ
INSTALL_DIR="$HOME/.local/bin"
CONFIG_DIR="$HOME/.config/ambient"

# スクリプトのディレクトリを取得
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# 1. 依存関係の確認
echo "依存関係を確認しています..."

# Rustの確認
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}✗ Rustが見つかりません${NC}"
    echo ""
    echo "Rustをインストールしてください:"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo ""
    exit 1
else
    echo -e "${GREEN}✓ Rust $(rustc --version | cut -d' ' -f2)${NC}"
fi

# Ollamaの確認
if ! command -v ollama &> /dev/null; then
    echo -e "${YELLOW}⚠ Ollamaが見つかりません${NC}"
    echo ""
    echo "Ambient WatcherはOllamaを使用してローカルLLMを実行します。"
    echo "インストール方法: https://ollama.ai"
    echo ""
    read -p "Ollamaなしで続行しますか？ (y/n): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
else
    echo -e "${GREEN}✓ Ollama $(ollama --version 2>/dev/null | head -1)${NC}"
    
    # モデルの確認
    if ollama list 2>/dev/null | grep -q "gpt-oss:20b"; then
        echo -e "${GREEN}✓ gpt-oss:20b モデル${NC}"
    else
        echo -e "${YELLOW}⚠ gpt-oss:20b モデルが見つかりません${NC}"
        echo "  推奨: ollama pull gpt-oss:20b"
    fi
fi

echo ""

# 2. ビルド
echo "Ambient Code Watcherをビルドしています..."
cd "$SCRIPT_DIR"
cargo build --release --bin codex
if [ $? -ne 0 ]; then
    echo -e "${RED}ビルドに失敗しました${NC}"
    exit 1
fi
echo -e "${GREEN}✓ ビルド完了${NC}"
echo ""

# 3. インストールディレクトリの作成
echo "インストールディレクトリを準備しています..."
mkdir -p "$INSTALL_DIR"
mkdir -p "$CONFIG_DIR"

# 4. 実行ファイルのインストール
echo "実行ファイルをインストールしています..."
# codexバイナリをコピー
cp "$SCRIPT_DIR/target/release/codex" "$INSTALL_DIR/codex-ambient"
chmod +x "$INSTALL_DIR/codex-ambient"

# 5. ambientラッパースクリプトを作成
cat > "$INSTALL_DIR/ambient" << 'EOF'
#!/bin/bash
# Ambient Code Watcher 起動スクリプト
export CODEX_OSS_BASE_URL="${CODEX_OSS_BASE_URL:-http://localhost:11434/v1}"
exec "$HOME/.local/bin/codex-ambient" ambient "$@"
EOF
chmod +x "$INSTALL_DIR/ambient"

echo -e "${GREEN}✓ 実行ファイルをインストールしました${NC}"
echo ""

# 6. UIファイルのコピー
echo "UIファイルをインストールしています..."
mkdir -p "$CONFIG_DIR/ui/static"
cp -r "$SCRIPT_DIR/cli/src/ambient_ui/index.html" "$CONFIG_DIR/ui/"
cp -r "$SCRIPT_DIR/cli/src/ambient_ui/static/"* "$CONFIG_DIR/ui/static/"
echo -e "${GREEN}✓ UIファイルをインストールしました${NC}"
echo ""

# 7. 設定ファイルのコピー
if [ ! -f "$CONFIG_DIR/config.toml" ]; then
    echo "設定ファイルを作成しています..."
    cat > "$CONFIG_DIR/config.toml" << 'EOF'
# Ambient Code Watcher 設定ファイル

# チェック間隔（秒）
check_interval_secs = 60

# WebUIのポート番号
port = 38080

# 監視対象のファイル拡張子
file_extensions = [
    "rs", "toml", "md", "txt", "yaml", "yml", "json",
    "js", "ts", "jsx", "tsx", "py", "go", "java", "c", "cpp", "h", "hpp"
]
EOF
    echo -e "${GREEN}✓ 設定ファイルを作成しました${NC}"
else
    echo -e "${YELLOW}⚠ 設定ファイルは既に存在します（スキップ）${NC}"
fi
echo ""

# 8. PATHの確認
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo -e "${YELLOW}PATHの設定が必要です${NC}"
    echo ""
    echo "以下のコマンドを実行するか、シェルの設定ファイルに追加してください:"
    echo ""
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo ""
    
    # シェルの設定ファイルを検出
    if [ -f "$HOME/.bashrc" ]; then
        echo "Bashを使用している場合:"
        echo "  echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.bashrc"
        echo "  source ~/.bashrc"
    fi
    if [ -f "$HOME/.zshrc" ]; then
        echo "Zshを使用している場合:"
        echo "  echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.zshrc"
        echo "  source ~/.zshrc"
    fi
else
    echo -e "${GREEN}✓ PATHは設定済みです${NC}"
fi
echo ""

# 9. 完了メッセージ
echo "========================================="
echo -e "${GREEN}インストールが完了しました！${NC}"
echo "========================================="
echo ""
echo "使い方:"
echo "  ambient          # Ambient Code Watcherを起動"
echo "  ambient --help   # ヘルプを表示"
echo ""
echo "設定ファイル:"
echo "  $CONFIG_DIR/config.toml"
echo ""
echo "WebUI:"
echo "  http://localhost:38080"
echo ""
echo "アンインストール:"
echo "  $SCRIPT_DIR/uninstall.sh"
echo ""