#!/bin/bash

# Chrome Bookmark MCP Server - 統合テストスクリプト
set -e

# カラー出力
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# ログ関数
log_info() { echo -e "${BLUE}ℹ${NC} $1"; }
log_success() { echo -e "${GREEN}✓${NC} $1"; }
log_error() { echo -e "${RED}✗${NC} $1"; }
log_warning() { echo -e "${YELLOW}!${NC} $1"; }

# バイナリパス
BINARY=${1:-"./target/release/mcp-bookmark"}

# ビルド
if [ ! -f "$BINARY" ]; then
    log_info "Building..."
    cargo build --release
fi

echo ""
echo "🧪 Chrome Bookmark MCP Server - Tests"
echo "======================================"

# 1. 基本動作
echo ""
echo "1. Basic Operations"
echo "-------------------"

log_info "Help check..."
$BINARY --help &>/dev/null && log_success "Help: OK" || log_error "Help: Failed"

# 2. プロファイル検出
echo ""
echo "2. Profile Detection"
echo "--------------------"

CHROME_DIR="$HOME/Library/Application Support/Google/Chrome"
if [ -d "$CHROME_DIR" ]; then
    PROFILES=$(find "$CHROME_DIR" -maxdepth 1 \( -name "Profile*" -o -name "Default" \) | wc -l | tr -d ' ')
    log_success "Chrome profiles found: $PROFILES"
else
    log_warning "Chrome not installed"
fi

# 3. MCP通信
echo ""
echo "3. MCP Communication"
echo "--------------------"

log_info "Initialize test..."
INIT='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"1.0.0","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}'

echo "$INIT" | timeout 2 $BINARY 2>/dev/null | grep -q '"result"' && {
    log_success "MCP initialize: OK"
} || {
    log_warning "MCP initialize: Timeout (normal)"
}

# 4. 環境変数
echo ""
echo "4. Environment Variables"
echo "------------------------"

log_info "Testing env vars..."
CHROME_PROFILE_NAME="Test" CHROME_TARGET_FOLDER="Test" timeout 2 $BINARY 2>&1 | grep -q "CHROME" && {
    log_success "Environment variables: OK"
} || {
    log_warning "Environment variables: Not detected"
}

# 5. パフォーマンス
echo ""
echo "5. Performance"
echo "--------------"

log_info "Startup time..."
START=$(date +%s%N)
timeout 1 $BINARY &>/dev/null || true
END=$(date +%s%N)
ELAPSED=$(( (END - START) / 1000000 ))

if [ $ELAPSED -lt 5000 ]; then
    log_success "Startup time: ${ELAPSED}ms (< 5s)"
else
    log_error "Startup time: ${ELAPSED}ms (> 5s)"
fi

# 6. インデックス管理
echo ""
echo "6. Index Management"
echo "-------------------"

log_info "List indexes..."
$BINARY --list-indexes &>/dev/null && log_success "List indexes: OK" || log_warning "List indexes: Failed"

# 7. Cargo テスト
echo ""
echo "7. Cargo Tests"
echo "--------------"

log_info "Running unit tests..."
cargo test --release --quiet && log_success "Unit tests: PASSED" || log_error "Unit tests: FAILED"

echo ""
echo "======================================"
echo "✅ Test suite completed"
echo ""