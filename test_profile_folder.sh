#!/bin/bash

# Test script for profile and folder configuration features

set -e

echo "🔍 Testing Chrome Bookmark MCP Server - Profile & Folder Configuration"
echo "======================================================================"

# Build the project first
echo "📦 Building the project..."
cargo build --release

BINARY="./target/release/mcp-bookmark"

echo ""
echo "1️⃣ Testing: List all available Chrome profiles"
echo "------------------------------------------------"
# This should show available profiles when no specific profile is selected
RUST_LOG=info $BINARY --help 2>&1 | head -20

echo ""
echo "2️⃣ Testing: Specify profile by name (Nakamura)"
echo "------------------------------------------------"
RUST_LOG=info timeout 2 $BINARY --profile "Nakamura" 2>&1 | head -20 || true

echo ""
echo "3️⃣ Testing: Specify profile and folder (Nakamura + hoge)"
echo "---------------------------------------------------------"
RUST_LOG=info timeout 2 $BINARY --profile "Nakamura" --folder "hoge" 2>&1 | head -20 || true

echo ""
echo "4️⃣ Testing: Using environment variables"
echo "----------------------------------------"
CHROME_PROFILE_NAME="Nakamura" CHROME_TARGET_FOLDER="hoge" RUST_LOG=info timeout 2 $BINARY 2>&1 | head -20 || true

echo ""
echo "5️⃣ Testing: Profile with folder and limit"
echo "------------------------------------------"
RUST_LOG=info timeout 2 $BINARY --profile "Nakamura" --folder "hoge" 10 2>&1 | head -20 || true

echo ""
echo "6️⃣ Testing: Invalid profile name"
echo "---------------------------------"
RUST_LOG=info timeout 2 $BINARY --profile "NonExistentProfile" 2>&1 | head -20 || true

echo ""
echo "✅ All tests completed!"
echo ""
echo "Note: The tests use 'timeout' to prevent the MCP server from running indefinitely."
echo "      Error messages about 'Timeout' are expected and normal."