# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## Project Overview

Chrome Bookmark MCP Server - provides read-only access to Chrome bookmarks via Model Context Protocol (MCP) for AI agents.

## Common Commands

```bash
# Build
cargo build --release

# Run
cargo run --release
cargo run --release -- Development 10  # フォルダ指定

# Index Management
./target/release/mcp-bookmark --list-indexes      # インデックス一覧
./target/release/mcp-bookmark --clear-index       # 現在設定のインデックスをクリア
./target/release/mcp-bookmark --clear-all-indexes # 全インデックスをクリア

# Lint and Format
cargo fmt
cargo clippy
```

## Architecture

- `src/bookmark.rs` - Chrome bookmark JSON parsing and filtering
- `src/content.rs` - Web page metadata fetching
- `src/config.rs` - Configuration management
- `src/main.rs` - MCP server entry point

## Key Features

- Auto-detect Chrome profile (largest bookmarks file)
- Folder filtering to reduce context
- Independent indexes per profile/folder combination (`Default_Development`, `Work_Tech_React`, etc.)
- Simple command-line interface
- macOS only (Chrome bookmarks at `~/Library/Application Support/Google/Chrome/*/Bookmarks`)

## Index Management

Indexes are stored at `~/Library/Application Support/mcp-bookmark/{profile}_{folder}/`
- Same profile/folder settings share the same index across projects
- Different settings use completely separate indexes