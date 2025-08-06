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
- Simple command-line interface
- macOS only (Chrome bookmarks at `~/Library/Application Support/Google/Chrome/*/Bookmarks`)