#!/bin/bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=================================${NC}"
echo -e "${BLUE}MCP Bookmark Installation Script${NC}"
echo -e "${BLUE}=================================${NC}"
echo

# Function to print colored messages
print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

# Check prerequisites
check_prerequisites() {
    echo -e "${BLUE}Checking prerequisites...${NC}"
    echo
    
    local prerequisites_met=true
    
    # Check OS
    if [[ "$OSTYPE" != "darwin"* ]]; then
        print_error "This script requires macOS"
        prerequisites_met=false
    else
        print_success "macOS detected"
    fi
    
    # Check Rust/Cargo
    if ! command -v cargo &> /dev/null; then
        print_error "Rust/Cargo not found"
        echo "  Please install Rust from https://rustup.rs"
        prerequisites_met=false
    else
        print_success "Rust/Cargo found ($(cargo --version))"
    fi
    
    # Check Chrome
    if [[ ! -d "$HOME/Library/Application Support/Google/Chrome" ]]; then
        print_error "Google Chrome not found"
        echo "  Please install Google Chrome from https://www.google.com/chrome/"
        prerequisites_met=false
    else
        print_success "Google Chrome found"
    fi
    
    # Check if we're in the right directory
    if [[ ! -f "Cargo.toml" ]]; then
        print_error "Please run this script from the mcp-bookmark project root directory"
        prerequisites_met=false
    else
        print_success "Project directory confirmed"
    fi
    
    if [ "$prerequisites_met" = false ]; then
        echo
        print_error "Prerequisites not met. Please install missing components and try again."
        exit 1
    fi
    
    echo
}

# Build binaries
build_binaries() {
    echo -e "${BLUE}Building binaries...${NC}"
    echo
    
    print_info "Building MCP server..."
    if cargo build --release --bin mcp-bookmark; then
        print_success "MCP server built successfully"
    else
        print_error "Failed to build MCP server"
        exit 1
    fi
    
    print_info "Building native messaging host..."
    if cargo build --release --bin mcp-bookmark-native; then
        print_success "Native messaging host built successfully"
    else
        print_error "Failed to build native messaging host"
        exit 1
    fi
    
    echo
}

# Setup Chrome extension
setup_extension() {
    echo -e "${BLUE}Setting up Chrome extension...${NC}"
    echo
    
    # Check if extension directory exists
    if [[ ! -d "bookmark-indexer-extension" ]]; then
        print_error "Chrome extension directory not found"
        exit 1
    fi
    
    print_info "Chrome extension found at: $(pwd)/bookmark-indexer-extension"
    echo
    print_warning "Please follow these steps to install the Chrome extension:"
    echo "  1. Open Chrome and navigate to chrome://extensions/"
    echo "  2. Enable 'Developer mode' (top right corner)"
    echo "  3. Click 'Load unpacked'"
    echo "  4. Select the directory: $(pwd)/bookmark-indexer-extension"
    echo "  5. Note the Extension ID that appears after loading"
    echo
    
    # Get extension ID from user
    read -p "Enter the Chrome Extension ID (found in chrome://extensions/): " EXT_ID
    
    if [[ -z "$EXT_ID" ]]; then
        print_error "Extension ID is required"
        exit 1
    fi
    
    # Setup native messaging host
    MANIFEST_DIR="$HOME/Library/Application Support/Google/Chrome/NativeMessagingHosts"
    MANIFEST_PATH="$MANIFEST_DIR/com.mcp_bookmark.json"
    
    print_info "Setting up native messaging host..."
    mkdir -p "$MANIFEST_DIR"
    
    cat > "$MANIFEST_PATH" <<EOF
{
  "name": "com.mcp_bookmark",
  "description": "MCP Bookmark Native Messaging Host",
  "path": "$(pwd)/target/release/mcp-bookmark-native",
  "type": "stdio",
  "allowed_origins": ["chrome-extension://$EXT_ID/"]
}
EOF
    
    print_success "Native messaging host configured at: $MANIFEST_PATH"
    echo
}

# Setup MCP configuration
setup_mcp_config() {
    echo -e "${BLUE}Setting up MCP configuration...${NC}"
    echo
    
    # Get initial index name
    print_info "Choose an initial index name for your bookmarks"
    echo "  Examples: 'work', 'personal', 'research', 'development'"
    read -p "Enter index name (default: Extension_Bookmarks): " INDEX_NAME
    
    if [[ -z "$INDEX_NAME" ]]; then
        INDEX_NAME="Extension_Bookmarks"
    fi
    
    # Validate index name
    if [[ ! "$INDEX_NAME" =~ ^[a-zA-Z0-9_]+$ ]]; then
        print_warning "Index name should only contain letters, numbers, and underscores"
        INDEX_NAME=$(echo "$INDEX_NAME" | sed 's/[^a-zA-Z0-9_]/_/g')
        print_info "Using sanitized name: $INDEX_NAME"
    fi
    
    # Create local .mcp.json file
    print_info "Creating local .mcp.json configuration file..."
    cat > .mcp.json <<EOF
{
  "mcpServers": {
    "mcp-bookmark": {
      "command": "$(pwd)/target/release/mcp-bookmark",
      "args": [],
      "env": {
        "RUST_LOG": "info",
        "INDEX_NAME": "$INDEX_NAME"
      }
    }
  }
}
EOF
    print_success "Created .mcp.json with INDEX_NAME: $INDEX_NAME"
    echo
}

# Verify installation
verify_installation() {
    echo -e "${BLUE}Verifying installation...${NC}"
    echo
    
    # Check binaries
    if [[ -f "target/release/mcp-bookmark" ]]; then
        print_success "MCP server binary found"
    else
        print_error "MCP server binary not found"
    fi
    
    if [[ -f "target/release/mcp-bookmark-native" ]]; then
        print_success "Native messaging host binary found"
    else
        print_error "Native messaging host binary not found"
    fi
    
    # Check native messaging manifest
    if [[ -f "$HOME/Library/Application Support/Google/Chrome/NativeMessagingHosts/com.mcp_bookmark.json" ]]; then
        print_success "Native messaging manifest found"
    else
        print_error "Native messaging manifest not found"
    fi
    
    echo
}

# Print next steps
print_next_steps() {
    echo -e "${GREEN}=================================${NC}"
    echo -e "${GREEN}Installation Complete!${NC}"
    echo -e "${GREEN}=================================${NC}"
    echo
    echo -e "${BLUE}Next steps:${NC}"
    echo "1. If you haven't already, load the Chrome extension from chrome://extensions/"
    echo "2. Click the extension icon in Chrome and create your first index"
    echo "3. Select a bookmark folder to index"
    echo "4. Test the MCP server with: INDEX_NAME=\"$INDEX_NAME\" ./target/release/mcp-bookmark"
    echo
    echo -e "${BLUE}Useful commands:${NC}"
    echo "  List all indexes:     ./target/release/mcp-bookmark --list-indexes"
    echo "  Clear an index:       ./target/release/mcp-bookmark --clear-index"
    echo "  Clear all indexes:    ./target/release/mcp-bookmark --clear-all-indexes"
    echo
    echo -e "${BLUE}Configuration:${NC}"
    echo "  Local MCP config:     .mcp.json (created with INDEX_NAME: $INDEX_NAME)"
    echo
    echo -e "${BLUE}Troubleshooting:${NC}"
    echo "  - If Chrome extension doesn't work, check the extension ID is correct"
    echo "  - If the server can't find bookmarks, verify INDEX_NAME matches your created index"
    echo "  - Run with debug logging: RUST_LOG=debug INDEX_NAME=\"$INDEX_NAME\" ./target/release/mcp-bookmark"
    echo
    print_info "For more help, see README.md or README.ja.md"
}

# Main installation flow
main() {
    check_prerequisites
    build_binaries
    setup_extension
    setup_mcp_config
    verify_installation
    print_next_steps
}

# Run main function
main