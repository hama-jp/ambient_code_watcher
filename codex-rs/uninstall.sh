#!/bin/bash
# Ambient Code Watcher アンインストールスクリプト

echo "========================================="
echo "  Ambient Code Watcher アンインストーラー"
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

# 確認
echo "以下のファイルを削除します:"
echo "  - $INSTALL_DIR/ambient"
echo "  - $INSTALL_DIR/codex-ambient"
echo "  - $CONFIG_DIR (設定ディレクトリ)"
echo ""
read -p "本当にアンインストールしますか？ (y/n): " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "アンインストールをキャンセルしました。"
    exit 0
fi

echo ""
echo "アンインストールを開始します..."

# 実行ファイルの削除
if [ -f "$INSTALL_DIR/ambient" ]; then
    rm "$INSTALL_DIR/ambient"
    echo -e "${GREEN}✓ ambientコマンドを削除しました${NC}"
else
    echo -e "${YELLOW}⚠ ambientコマンドが見つかりません${NC}"
fi

if [ -f "$INSTALL_DIR/codex-ambient" ]; then
    rm "$INSTALL_DIR/codex-ambient"
    echo -e "${GREEN}✓ codex-ambientを削除しました${NC}"
else
    echo -e "${YELLOW}⚠ codex-ambientが見つかりません${NC}"
fi


# 設定ファイルの削除
echo ""
read -p "設定ファイルも削除しますか？ (y/n): " -n 1 -r
echo ""

if [[ $REPLY =~ ^[Yy]$ ]]; then
    if [ -d "$CONFIG_DIR" ]; then
        rm -rf "$CONFIG_DIR"
        echo -e "${GREEN}✓ 設定ディレクトリを削除しました${NC}"
    else
        echo -e "${YELLOW}⚠ 設定ディレクトリが見つかりません${NC}"
    fi
else
    echo "設定ファイルは保持されます。"
fi

echo ""
echo "========================================="
echo -e "${GREEN}アンインストールが完了しました${NC}"
echo "========================================="
echo ""
echo "Ambient Code Watcherをご利用いただきありがとうございました！"
echo ""
echo "再インストールする場合:"
echo "  ./install.sh"
echo ""