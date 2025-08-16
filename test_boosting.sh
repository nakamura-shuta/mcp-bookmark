#!/bin/bash

echo "Testing Field Boosting (Phase 1.2)"
echo "==================================="
echo

# Create test HTML files
mkdir -p /tmp/test_bookmarks

# 1. Title match only
cat > /tmp/test_bookmarks/title_only.html << 'EOF'
<!DOCTYPE html>
<html>
<head><title>RDS Configuration Guide</title></head>
<body>This page contains general database information without mentioning the specific term.</body>
</html>
EOF

# 2. Content match only
cat > /tmp/test_bookmarks/content_only.html << 'EOF'
<!DOCTYPE html>
<html>
<head><title>Database Setup</title></head>
<body>This guide explains how to set up RDS instances in AWS. RDS provides managed database services.</body>
</html>
EOF

# 3. Both title and content match
cat > /tmp/test_bookmarks/both_match.html << 'EOF'
<!DOCTYPE html>
<html>
<head><title>RDS Best Practices</title></head>
<body>Learn about RDS configuration, RDS backup strategies, and RDS performance tuning.</body>
</html>
EOF

# 4. All fields match (title, URL, content)
cat > /tmp/test_bookmarks/all_match.html << 'EOF'
<!DOCTYPE html>
<html>
<head><title>Complete RDS Tutorial</title></head>
<body>Comprehensive guide for AWS RDS setup, RDS monitoring, and RDS optimization techniques.</body>
</html>
EOF

echo "Test cases created:"
echo "1. Title match only: 'RDS Configuration Guide' (expected score: ~3x)"
echo "2. Content match only: mentions RDS in body (expected score: ~1x)"
echo "3. Title + Content: 'RDS Best Practices' with RDS in content (expected score: ~4x)"
echo "4. All fields: Title + URL + Content all contain RDS (expected score: ~6x)"
echo
echo "Note: URL matching would apply if the URL contains 'rds' (e.g., aws.com/rds/guide)"
echo
echo "In real usage:"
echo "- The document with ALL matches would rank highest"
echo "- Title-only matches would rank above content-only matches"
echo "- Multiple field matches accumulate scores"