//! Tests for chunk-based content transfer (Chrome Native Messaging 1MB limit workaround)
//!
//! These tests verify that large content can be split into chunks and reassembled correctly.

use std::collections::HashMap;

/// Simulates the chunk splitting logic from background.js
fn split_content_into_chunks(content: &str, max_chunk_bytes: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current_pos = 0;
    let chars: Vec<char> = content.chars().collect();

    while current_pos < chars.len() {
        // Binary search for the right character count that fits in max_chunk_bytes
        let mut left = current_pos;
        let mut right = chars.len();

        while left < right {
            let mid = (left + right).div_ceil(2);
            let test_str: String = chars[current_pos..mid].iter().collect();
            let test_bytes = test_str.len();

            if test_bytes <= max_chunk_bytes {
                left = mid;
            } else {
                right = mid - 1;
            }
        }

        let chunk_content: String = chars[current_pos..left].iter().collect();
        chunks.push(chunk_content);
        current_pos = left;
    }

    chunks
}

/// Simulates the chunk reassembly logic from mcp-bookmark-native.rs
fn reassemble_chunks(chunks: &HashMap<usize, String>, total_chunks: usize) -> Option<String> {
    let mut full_content = String::new();

    for i in 0..total_chunks {
        if let Some(chunk) = chunks.get(&i) {
            full_content.push_str(chunk);
        } else {
            return None; // Missing chunk
        }
    }

    Some(full_content)
}

#[test]
fn test_split_and_reassemble_ascii_content() {
    // 700KB max chunk size (matching background.js)
    let max_chunk_bytes = 700 * 1024;

    // Create 1.5MB of ASCII content
    let content: String = "A".repeat(1_500_000);
    let original_len = content.len();

    // Split into chunks
    let chunks = split_content_into_chunks(&content, max_chunk_bytes);

    // Should need 3 chunks for 1.5MB content with 700KB max
    assert!(chunks.len() >= 2, "Should split into multiple chunks");
    assert!(chunks.len() <= 3, "Should not over-split");

    // Verify each chunk is within size limit
    for (i, chunk) in chunks.iter().enumerate() {
        let chunk_bytes = chunk.len();
        assert!(
            chunk_bytes <= max_chunk_bytes,
            "Chunk {i} exceeds max size: {chunk_bytes} > {max_chunk_bytes}"
        );
    }

    // Reassemble
    let mut chunk_map = HashMap::new();
    for (i, chunk) in chunks.iter().enumerate() {
        chunk_map.insert(i, chunk.clone());
    }

    let reassembled = reassemble_chunks(&chunk_map, chunks.len()).unwrap();

    // Verify content is preserved
    assert_eq!(reassembled.len(), original_len);
    assert_eq!(reassembled, content);
}

#[test]
fn test_split_and_reassemble_japanese_content() {
    // 700KB max chunk size
    let max_chunk_bytes = 700 * 1024;

    // Create Japanese content (3 bytes per char in UTF-8)
    // 300,000 chars * 3 bytes = 900KB (should need 2 chunks)
    let japanese_text = "日本語テキスト";
    let content: String = japanese_text.repeat(50_000); // About 350KB * 3 = 1MB+
    let original_len = content.len();
    let original_chars: Vec<char> = content.chars().collect();
    let original_char_count = original_chars.len();

    println!(
        "Content length: {} chars, {} bytes",
        original_char_count,
        content.len()
    );

    // Split into chunks
    let chunks = split_content_into_chunks(&content, max_chunk_bytes);

    println!("Split into {} chunks", chunks.len());

    // Should need at least 2 chunks
    assert!(
        chunks.len() >= 2,
        "Should split into multiple chunks for large Japanese content"
    );

    // Verify each chunk is within byte size limit
    for (i, chunk) in chunks.iter().enumerate() {
        let chunk_bytes = chunk.len();
        println!(
            "Chunk {}: {} chars, {} bytes",
            i,
            chunk.chars().count(),
            chunk_bytes
        );
        assert!(
            chunk_bytes <= max_chunk_bytes,
            "Chunk {i} exceeds max size: {chunk_bytes} > {max_chunk_bytes}"
        );
    }

    // Reassemble
    let mut chunk_map = HashMap::new();
    for (i, chunk) in chunks.iter().enumerate() {
        chunk_map.insert(i, chunk.clone());
    }

    let reassembled = reassemble_chunks(&chunk_map, chunks.len()).unwrap();

    // Verify content is preserved
    assert_eq!(
        reassembled.len(),
        original_len,
        "Character length should match"
    );
    assert_eq!(
        reassembled.chars().count(),
        original_char_count,
        "Character count should match"
    );
    assert_eq!(reassembled, content, "Content should be identical");
}

#[test]
fn test_split_and_reassemble_mixed_content() {
    // 700KB max chunk size
    let max_chunk_bytes = 700 * 1024;

    // Create mixed content with ASCII and Japanese
    let mixed =
        "Hello World! これは日本語のテストです。ASCII text here. 漢字とひらがなとカタカナ。";
    let content: String = mixed.repeat(20_000); // Create large content
    let original = content.clone();

    println!(
        "Mixed content: {} chars, {} bytes",
        content.chars().count(),
        content.len()
    );

    // Split into chunks
    let chunks = split_content_into_chunks(&content, max_chunk_bytes);

    println!("Split into {} chunks", chunks.len());

    // Verify each chunk is valid UTF-8 and within size limit
    for (i, chunk) in chunks.iter().enumerate() {
        let chunk_bytes = chunk.len();
        assert!(
            chunk_bytes <= max_chunk_bytes,
            "Chunk {i} exceeds max size: {chunk_bytes} > {max_chunk_bytes}"
        );

        // Verify it's valid UTF-8 (should always pass since we're using String)
        assert!(
            std::str::from_utf8(chunk.as_bytes()).is_ok(),
            "Chunk {i} is not valid UTF-8"
        );
    }

    // Reassemble
    let mut chunk_map = HashMap::new();
    for (i, chunk) in chunks.iter().enumerate() {
        chunk_map.insert(i, chunk.clone());
    }

    let reassembled = reassemble_chunks(&chunk_map, chunks.len()).unwrap();

    // Verify content is preserved
    assert_eq!(
        reassembled, original,
        "Reassembled content should match original"
    );
}

#[test]
fn test_small_content_no_split() {
    let max_chunk_bytes = 700 * 1024;

    // Small content that doesn't need splitting
    let content = "This is a small piece of content.";

    let chunks = split_content_into_chunks(content, max_chunk_bytes);

    assert_eq!(chunks.len(), 1, "Small content should not be split");
    assert_eq!(
        chunks[0], content,
        "Single chunk should contain entire content"
    );
}

#[test]
fn test_reassemble_with_missing_chunk() {
    let mut chunk_map = HashMap::new();
    chunk_map.insert(0, "First chunk".to_string());
    chunk_map.insert(2, "Third chunk".to_string());
    // Missing chunk 1

    let result = reassemble_chunks(&chunk_map, 3);

    assert!(result.is_none(), "Should fail when chunk is missing");
}

#[test]
fn test_exact_boundary_split() {
    // Test with content that's exactly at the chunk boundary
    let max_chunk_bytes = 100; // Small size for testing
    let content = "A".repeat(100); // Exactly 100 bytes

    let chunks = split_content_into_chunks(&content, max_chunk_bytes);

    assert_eq!(
        chunks.len(),
        1,
        "Content at exact boundary should be single chunk"
    );
    assert_eq!(chunks[0].len(), 100);

    // One more byte should cause split
    let content_plus_one = "A".repeat(101);
    let chunks_plus = split_content_into_chunks(&content_plus_one, max_chunk_bytes);

    assert_eq!(chunks_plus.len(), 2, "Content over boundary should split");
}

#[test]
fn test_pdf_like_content_with_page_markers() {
    let max_chunk_bytes = 700 * 1024;

    // Simulate PDF content with page markers (like offscreen.js produces)
    let mut pages = Vec::new();
    for page_num in 1..=100 {
        let page_content = format!(
            "[PAGE:{page_num}]\nこれはページ{page_num}のコンテンツです。This is page {page_num} content. Some more text here to make it larger. "
        );
        pages.push(page_content.repeat(100)); // Make each page substantial
    }

    let content = pages.join("\n\n");
    let original = content.clone();

    println!(
        "PDF-like content: {} chars, {} bytes",
        content.chars().count(),
        content.len()
    );

    // Split into chunks
    let chunks = split_content_into_chunks(&content, max_chunk_bytes);

    println!("Split into {} chunks", chunks.len());

    // Reassemble
    let mut chunk_map = HashMap::new();
    for (i, chunk) in chunks.iter().enumerate() {
        chunk_map.insert(i, chunk.clone());
    }

    let reassembled = reassemble_chunks(&chunk_map, chunks.len()).unwrap();

    // Verify all page markers are preserved
    for page_num in 1..=100 {
        let marker = format!("[PAGE:{page_num}]");
        assert!(
            reassembled.contains(&marker),
            "Page marker {marker} should be preserved after reassembly"
        );
    }

    assert_eq!(reassembled, original, "Full content should be preserved");
}

#[test]
fn test_chunk_order_independence() {
    let max_chunk_bytes = 100;
    let content = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".repeat(10); // 260 bytes

    let chunks = split_content_into_chunks(&content, max_chunk_bytes);

    // Insert chunks in reverse order
    let mut chunk_map = HashMap::new();
    for (i, chunk) in chunks.iter().enumerate().rev() {
        chunk_map.insert(i, chunk.clone());
    }

    let reassembled = reassemble_chunks(&chunk_map, chunks.len()).unwrap();

    assert_eq!(reassembled, content, "Order of insertion should not matter");
}
