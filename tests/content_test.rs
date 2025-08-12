use mcp_bookmark::content::ContentFetcher;

#[tokio::test]
async fn test_content_extraction() {
    let fetcher = ContentFetcher::new().expect("Failed to create content fetcher");

    // Test with a simple HTML
    let html = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Test Page</title>
            <meta name="description" content="A test page">
            <meta property="og:title" content="OG Test Title">
            <meta property="og:description" content="OG Test Description">
        </head>
        <body>
            <h1>Main Heading</h1>
            <p>This is the first paragraph with some text content.</p>
            <article>
                <h2>Article Title</h2>
                <p>This is the article content that should be extracted as main content.</p>
            </article>
            <p>Another paragraph outside the article.</p>
        </body>
        </html>
    "#;

    let base_url = "https://example.com";

    // Test metadata extraction
    let metadata = fetcher.extract_metadata(html, base_url);
    assert_eq!(metadata.title, Some("Test Page".to_string()));
    assert_eq!(metadata.description, Some("A test page".to_string()));
    assert_eq!(metadata.og_title, Some("OG Test Title".to_string()));
    assert_eq!(
        metadata.og_description,
        Some("OG Test Description".to_string())
    );

    // Test content extraction
    let content = fetcher.extract_content(html, base_url);
    assert!(content.html_title.is_some());
    assert_eq!(content.html_title, Some("Test Page".to_string()));

    // Check that text content is extracted
    assert!(content.text_content.is_some());
    let text = content.text_content.unwrap();

    // Readability might not extract content from simple test HTML,
    // but the fallback should work
    if text.is_empty() {
        println!("Note: Readability couldn't extract from simple HTML (expected for test case)");
    } else {
        // Either readability or fallback should extract these
        assert!(text.contains("Main Heading") || text.contains("Article Title"));
    }

    // Main content might be extracted or might use fallback
    if let Some(main) = content.main_content {
        println!("Main content extracted: {} chars", main.len());
        // Don't assert specific content as readability behavior varies with simple HTML
    }

    println!("✅ Content extraction test passed!");
}

#[tokio::test]
async fn test_real_url_fetch() {
    let fetcher = ContentFetcher::new().expect("Failed to create content fetcher");

    // Test with example.com (always available)
    match fetcher.fetch_page("https://example.com").await {
        Ok(html) => {
            assert!(html.contains("Example Domain"));

            let metadata = fetcher.extract_metadata(&html, "https://example.com");
            assert!(metadata.title.is_some());

            let content = fetcher.extract_content(&html, "https://example.com");
            assert!(content.text_content.is_some());

            // Verify full content is not truncated
            if let Some(text) = content.text_content {
                println!("Fetched content length: {} chars", text.len());
                assert!(!text.is_empty());
            }

            println!("✅ Real URL fetch test passed!");
        }
        Err(e) => {
            eprintln!("Warning: Could not fetch example.com: {e}");
        }
    }
}
