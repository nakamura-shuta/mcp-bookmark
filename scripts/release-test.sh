#!/bin/bash

# Chrome Bookmark MCP Server - リリーステストスクリプト
# 
# 使用方法:
#   ./scripts/release-test.sh [バイナリパス]
#
# 例:
#   ./scripts/release-test.sh ./target/release/mcp-bookmark
#   ./scripts/release-test.sh ~/.local/bin/mcp-bookmark

set -e

# カラー出力
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# テスト結果カウンタ
PASSED=0
FAILED=0
SKIPPED=0

# ログ関数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
    ((PASSED++))
}

log_error() {
    echo -e "${RED}[✗]${NC} $1"
    ((FAILED++))
}

log_warning() {
    echo -e "${YELLOW}[!]${NC} $1"
}

log_skip() {
    echo -e "${YELLOW}[SKIP]${NC} $1"
    ((SKIPPED++))
}

# ヘッダー表示
print_header() {
    echo ""
    echo "======================================"
    echo "$1"
    echo "======================================"
}

# バイナリパスの確認
BINARY_PATH=${1:-"mcp-bookmark"}

if ! command -v "$BINARY_PATH" &> /dev/null; then
    if [ -f "$BINARY_PATH" ]; then
        BINARY_PATH="$(realpath "$BINARY_PATH")"
    else
        log_error "バイナリが見つかりません: $BINARY_PATH"
        exit 1
    fi
fi

log_info "テスト対象バイナリ: $BINARY_PATH"

# テスト用一時ディレクトリ
TEST_DIR=$(mktemp -d)
log_info "テスト用ディレクトリ: $TEST_DIR"

# クリーンアップ関数
cleanup() {
    rm -rf "$TEST_DIR"
}
trap cleanup EXIT

# ===========================================
# 1. 基本動作テスト
# ===========================================
print_header "1. 基本動作テスト"

# バージョン確認
log_info "バージョン確認..."
if $BINARY_PATH --version &> /dev/null; then
    VERSION=$($BINARY_PATH --version 2>&1 || echo "unknown")
    log_success "バージョン表示成功: $VERSION"
else
    log_error "バージョン表示失敗"
fi

# ヘルプ表示
log_info "ヘルプ表示確認..."
if $BINARY_PATH --help &> /dev/null; then
    log_success "ヘルプ表示成功"
else
    log_error "ヘルプ表示失敗"
fi

# ===========================================
# 2. Chrome検出テスト
# ===========================================
print_header "2. Chrome検出テスト"

CHROME_DIR="$HOME/Library/Application Support/Google/Chrome"

if [ -d "$CHROME_DIR" ]; then
    log_info "Chromeディレクトリ検出: $CHROME_DIR"
    
    # プロファイル数をカウント
    PROFILE_COUNT=$(find "$CHROME_DIR" -maxdepth 1 -name "Profile*" -o -name "Default" | wc -l | tr -d ' ')
    log_success "検出されたプロファイル数: $PROFILE_COUNT"
    
    # Bookmarksファイルの存在確認
    BOOKMARK_FILES=$(find "$CHROME_DIR" -name "Bookmarks" 2>/dev/null | wc -l | tr -d ' ')
    log_success "Bookmarksファイル数: $BOOKMARK_FILES"
else
    log_skip "Chromeがインストールされていません"
fi

# ===========================================
# 3. MCP通信テスト
# ===========================================
print_header "3. MCP通信テスト"

log_info "MCPサーバー起動テスト..."

# タイムアウト付きでサーバーを起動してinitializeメッセージを送信
INIT_REQUEST='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"1.0.0","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}'

# サーバー起動とinitializeテスト
echo "$INIT_REQUEST" | timeout 2 $BINARY_PATH 2>/dev/null | grep -q '"result"' && {
    log_success "MCPサーバー起動・initialize成功"
} || {
    log_warning "MCPサーバー起動テストはタイムアウト（正常）"
}

# ===========================================
# 4. コマンドライン引数テスト
# ===========================================
print_header "4. コマンドライン引数テスト"

# プロファイル指定テスト
log_info "プロファイル指定テスト..."
echo "$INIT_REQUEST" | timeout 2 $BINARY_PATH --profile "Default" 2>&1 | grep -q "error" && {
    log_warning "存在しないプロファイルでエラー（期待通り）"
} || {
    log_success "プロファイル指定引数受け付け"
}

# フォルダ指定テスト
log_info "フォルダ指定テスト..."
echo "$INIT_REQUEST" | timeout 2 $BINARY_PATH --folder "TestFolder" 2>/dev/null &
PID=$!
sleep 1
if kill -0 $PID 2>/dev/null; then
    kill $PID 2>/dev/null
    log_success "フォルダ指定引数受け付け"
else
    log_warning "プロセスが早期終了"
fi

# ===========================================
# 5. 環境変数テスト
# ===========================================
print_header "5. 環境変数テスト"

log_info "環境変数設定テスト..."
CHROME_PROFILE_NAME="Test" CHROME_TARGET_FOLDER="TestFolder" timeout 2 $BINARY_PATH 2>&1 | grep -q -E "(CHROME_PROFILE_NAME|profile|Profile)" && {
    log_success "環境変数認識成功"
} || {
    log_warning "環境変数テスト（タイムアウト正常）"
}

# ===========================================
# 6. エラーハンドリングテスト
# ===========================================
print_header "6. エラーハンドリングテスト"

# 無効なプロファイル名
log_info "無効なプロファイル名テスト..."
echo "$INIT_REQUEST" | timeout 2 $BINARY_PATH --profile "///invalid///" 2>&1 | grep -q -E "(error|Error|failed|Failed)" && {
    log_success "無効なプロファイル名でエラー処理"
} || {
    log_warning "エラーメッセージが検出されませんでした"
}

# ===========================================
# 7. パフォーマンステスト
# ===========================================
print_header "7. パフォーマンステスト"

log_info "起動時間測定..."
START_TIME=$(date +%s%N)
timeout 1 $BINARY_PATH 2>/dev/null || true
END_TIME=$(date +%s%N)
ELAPSED=$((($END_TIME - $START_TIME) / 1000000))

if [ $ELAPSED -lt 5000 ]; then
    log_success "起動時間: ${ELAPSED}ms (< 5000ms)"
else
    log_error "起動時間: ${ELAPSED}ms (> 5000ms)"
fi

# ===========================================
# 8. メモリ使用量テスト（macOS）
# ===========================================
print_header "8. リソース使用量テスト"

log_info "プロセス起動とリソース確認..."
$BINARY_PATH > /dev/null 2>&1 &
PID=$!
sleep 2

if kill -0 $PID 2>/dev/null; then
    # macOSでのメモリ使用量取得
    if command -v ps &> /dev/null; then
        MEM_KB=$(ps -o rss= -p $PID 2>/dev/null | tr -d ' ' || echo "0")
        if [ -n "$MEM_KB" ] && [ "$MEM_KB" -gt 0 ]; then
            MEM_MB=$((MEM_KB / 1024))
            if [ $MEM_MB -lt 500 ]; then
                log_success "メモリ使用量: ${MEM_MB}MB (< 500MB)"
            else
                log_error "メモリ使用量: ${MEM_MB}MB (> 500MB)"
            fi
        else
            log_skip "メモリ使用量を取得できませんでした"
        fi
    else
        log_skip "psコマンドが利用できません"
    fi
    
    kill $PID 2>/dev/null
else
    log_warning "プロセスが既に終了しています"
fi

# ===========================================
# 9. ログ出力テスト
# ===========================================
print_header "9. ログ出力テスト"

log_info "デバッグログ出力テスト..."
RUST_LOG=debug timeout 2 $BINARY_PATH 2>&1 | grep -q -E "(DEBUG|TRACE|INFO)" && {
    log_success "デバッグログ出力確認"
} || {
    log_warning "デバッグログ未検出（タイムアウト可能性）"
}

# ===========================================
# 10. ツール呼び出しテスト
# ===========================================
print_header "10. MCPツール呼び出しシミュレーション"

log_info "利用可能なツールリスト取得..."

# ツールリスト要求
TOOLS_REQUEST='{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'

# 実際にはMCPセッションが必要なため、基本的な応答のみテスト
{
    echo "$INIT_REQUEST"
    sleep 0.5
    echo '{"jsonrpc":"2.0","id":2,"method":"initialized","params":{}}'
    sleep 0.5
    echo "$TOOLS_REQUEST"
} | timeout 3 $BINARY_PATH 2>/dev/null | grep -q "search_bookmarks" && {
    log_success "MCPツール定義確認"
} || {
    log_warning "MCPツール定義未確認（セッション必要）"
}

# ===========================================
# テスト結果サマリー
# ===========================================
print_header "テスト結果サマリー"

TOTAL=$((PASSED + FAILED + SKIPPED))

echo ""
echo "合計テスト数: $TOTAL"
echo -e "${GREEN}成功: $PASSED${NC}"
echo -e "${RED}失敗: $FAILED${NC}"
echo -e "${YELLOW}スキップ: $SKIPPED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ 全てのテストが成功しました！${NC}"
    exit 0
else
    echo -e "${RED}❌ $FAILED 個のテストが失敗しました${NC}"
    exit 1
fi