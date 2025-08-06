#!/bin/bash
# GitHub Actions release workflow のローカルテスト

set -e

echo "📦 リリースビルドのテスト"
echo "========================="

# 現在のOS/アーキテクチャを検出
OS=$(uname -s)
ARCH=$(uname -m)

case "$OS" in
    Darwin)
        if [[ "$ARCH" == "arm64" ]]; then
            TARGET="aarch64-apple-darwin"
            BINARY_NAME="mcp-bookmark-darwin-arm64"
        else
            TARGET="x86_64-apple-darwin"
            BINARY_NAME="mcp-bookmark-darwin-x64"
        fi
        ;;
    Linux)
        TARGET="x86_64-unknown-linux-gnu"
        BINARY_NAME="mcp-bookmark-linux-x64"
        ;;
    *)
        echo "❌ サポートされていないOS: $OS"
        exit 1
        ;;
esac

echo "🎯 ターゲット: $TARGET"
echo "📝 バイナリ名: $BINARY_NAME"

# ターゲットを追加
echo "🔧 Rustターゲットを追加..."
rustup target add $TARGET 2>/dev/null || true

# ビルド
echo "🔨 ビルド中..."
cargo build --release --target $TARGET

# バイナリをコピー
echo "📋 バイナリをコピー..."
cp target/$TARGET/release/mcp-bookmark ./$BINARY_NAME
chmod +x ./$BINARY_NAME

# サイズ確認
SIZE=$(ls -lh ./$BINARY_NAME | awk '{print $5}')
echo "✅ ビルド成功！"
echo "📊 バイナリサイズ: $SIZE"

# テスト実行
echo ""
echo "🧪 動作テスト..."
./$BINARY_NAME --help

echo ""
echo "✨ 完了！"
echo "生成されたバイナリ: ./$BINARY_NAME"