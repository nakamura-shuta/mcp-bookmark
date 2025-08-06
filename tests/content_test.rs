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
    let content = fetcher.extract_content(html);
    assert!(content.html_title.is_some());
    assert_eq!(content.html_title, Some("Test Page".to_string()));

    // Check that text content is extracted
    assert!(content.text_content.is_some());
    let text = content.text_content.unwrap();
    assert!(text.contains("Main Heading"));
    assert!(text.contains("first paragraph"));
    assert!(text.contains("Article Title"));
    assert!(text.contains("article content"));

    // Check that main content (article) is extracted
    assert!(content.main_content.is_some());
    let main = content.main_content.unwrap();
    assert!(main.contains("Article Title"));
    assert!(main.contains("article content"));

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

            let content = fetcher.extract_content(&html);
            assert!(content.text_content.is_some());

            // Verify full content is not truncated
            if let Some(text) = content.text_content {
                println!("Fetched content length: {} chars", text.len());
                assert!(text.len() > 0);
            }

            println!("✅ Real URL fetch test passed!");
        }
        Err(e) => {
            eprintln!("Warning: Could not fetch example.com: {}", e);
        }
    }
}
