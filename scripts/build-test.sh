#!/bin/bash

# ビルドテストスクリプト（macOS専用）
set -e

echo "📦 Build Test for macOS"
echo "======================="

# OS/アーキテクチャ検出
OS=$(uname -s)
ARCH=$(uname -m)

if [ "$OS" != "Darwin" ]; then
    echo "❌ This tool is macOS only (detected: $OS)"
    exit 1
fi

# ターゲット設定
if [ "$ARCH" == "arm64" ]; then
    TARGET="aarch64-apple-darwin"
    BINARY_NAME="mcp-bookmark-darwin-arm64"
else
    TARGET="x86_64-apple-darwin"
    BINARY_NAME="mcp-bookmark-darwin-x64"
fi

echo "🎯 Target: $TARGET"
echo "📝 Binary: $BINARY_NAME"

# ビルド（既にビルド済みならスキップ）
if [ -f "target/$TARGET/release/mcp-bookmark" ]; then
    echo "ℹ️  Using existing build"
else
    echo "🔨 Building..."
    cargo build --release --target $TARGET
fi

# バイナリコピー
cp target/$TARGET/release/mcp-bookmark ./$BINARY_NAME
chmod +x ./$BINARY_NAME

# サイズ確認
SIZE=$(ls -lh ./$BINARY_NAME | awk '{print $5}')
echo "✅ Build success! Size: $SIZE"

# 動作確認
echo "🧪 Testing..."
./$BINARY_NAME --help | head -1

echo "✨ Complete! Binary: ./$BINARY_NAME"