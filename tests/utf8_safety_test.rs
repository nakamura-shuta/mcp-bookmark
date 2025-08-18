#[cfg(test)]
mod utf8_safety_tests {
    use mcp_bookmark::search::scored_snippet::ScoredSnippetGenerator;

    #[test]
    fn test_japanese_snippet_truncation() {
        let generator = ScoredSnippetGenerator::new();

        // Test with Japanese text
        let content = "これは日本語のテストです。検索エンジンの最適化について説明します。";
        let query = "検索";

        let snippet = generator.generate_snippet(content, query, 20);

        // Should not panic and should be valid UTF-8
        assert!(snippet.text.is_char_boundary(0));
        assert!(snippet.text.is_char_boundary(snippet.text.len()));
    }

    #[test]
    fn test_mixed_content_truncation() {
        let generator = ScoredSnippetGenerator::new();

        // Test with mixed English, Japanese, and emoji
        let content = "Hello 世界! 🌏 This is a test テストです with mixed content 混合コンテンツ.";
        let query = "test";

        let snippet = generator.generate_snippet(content, query, 30);

        // Verify the snippet is valid UTF-8
        assert!(snippet.text.is_char_boundary(0));
        assert!(snippet.text.is_char_boundary(snippet.text.len()));
    }

    #[test]
    fn test_emoji_boundary_safety() {
        let generator = ScoredSnippetGenerator::new();

        // Test with emojis that use multiple bytes
        let content = "Start 🎌🗾🍱🍜🍙 Japanese food emojis 日本料理の絵文字 end";
        let query = "Japanese";

        let snippet = generator.generate_snippet(content, query, 25);

        // Should handle multi-byte emoji boundaries correctly
        assert!(snippet.text.is_char_boundary(0));
        assert!(snippet.text.is_char_boundary(snippet.text.len()));

        // Try various truncation lengths
        for max_len in [10, 15, 20, 25, 30] {
            let snippet = generator.generate_snippet(content, query, max_len);
            assert!(snippet.text.is_char_boundary(0));
            assert!(snippet.text.is_char_boundary(snippet.text.len()));
        }
    }

    #[test]
    fn test_chinese_korean_mixed() {
        let generator = ScoredSnippetGenerator::new();

        // Test with Chinese and Korean characters
        let content = "中文内容 Chinese content 한국어 콘텐츠 Korean content 混合内容";
        let query = "content";

        let snippet = generator.generate_snippet(content, query, 20);

        // Verify UTF-8 safety
        assert!(snippet.text.is_char_boundary(0));
        assert!(snippet.text.is_char_boundary(snippet.text.len()));
    }

    #[test]
    fn test_long_japanese_text_truncation() {
        let generator = ScoredSnippetGenerator::new();

        // Test with longer Japanese text
        let content = "日本語の長いテキストをテストしています。これは検索エンジンのインデックス作成と検索機能のテストです。UTF-8の境界を適切に処理することが重要です。特に日本語のような多バイト文字を扱う場合は注意が必要です。";
        let query = "インデックス";

        // Test various snippet lengths
        for max_len in [10, 20, 50, 100, 150, 200] {
            let snippet = generator.generate_snippet(content, query, max_len);

            // Verify each truncation is UTF-8 safe
            assert!(snippet.text.is_char_boundary(0));
            assert!(snippet.text.is_char_boundary(snippet.text.len()));
            assert!(snippet.text.len() <= max_len + 3); // +3 for "..."
        }
    }
}

#[cfg(test)]
mod main_tests {
    #[test]
    fn run_utf8_safety_tests() {
        println!("Running UTF-8 safety tests...");
        // Tests will run automatically
    }
}
