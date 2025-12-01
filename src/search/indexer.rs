use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use tantivy::{Index, IndexWriter, TantivyDocument};
use tracing::{debug, warn};

/// Log to file for debugging in native messaging context
fn log_to_file_indexer(message: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/mcp-bookmark-indexer.log")
    {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = writeln!(file, "[{timestamp}] {message}");
    }
}

use super::common::{DEFAULT_WRITER_HEAP_SIZE, MIN_WRITER_HEAP_SIZE, extract_domain, parse_date};
use super::schema::BookmarkSchema;
use crate::bookmark::FlatBookmark;

/// Page information for chunked content (PDFs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageInfo {
    pub page_count: usize,
    pub page_offsets: Vec<usize>,
    pub content_type: String,
    pub total_chars: usize,
}

/// Handles indexing operations for bookmarks
#[derive(Debug)]
pub struct BookmarkIndexer {
    index: Index,
    schema: BookmarkSchema,
}

impl BookmarkIndexer {
    /// Create a new indexer
    pub fn new(index: Index, schema: BookmarkSchema) -> Self {
        Self { index, schema }
    }

    /// Get a reference to the schema
    pub fn schema(&self) -> &BookmarkSchema {
        &self.schema
    }

    /// Get a reference to the index
    pub fn index(&self) -> &Index {
        &self.index
    }

    /// Create an index writer
    pub fn create_writer(&self, heap_size: usize) -> Result<IndexWriter> {
        // Ensure minimum heap size for tantivy 0.24
        let actual_heap = heap_size.max(MIN_WRITER_HEAP_SIZE);
        self.index
            .writer(actual_heap)
            .context("Failed to create index writer")
    }

    /// Index a single bookmark
    pub fn index_bookmark(
        &self,
        writer: &mut IndexWriter,
        bookmark: &FlatBookmark,
        content: Option<&str>,
    ) -> Result<()> {
        let doc = self.create_document(bookmark, content, None)?;
        writer.add_document(doc)?;
        Ok(())
    }

    /// Index a single bookmark with page information
    pub fn index_bookmark_with_page_info(
        &self,
        writer: &mut IndexWriter,
        bookmark: &FlatBookmark,
        content: Option<&str>,
        page_info: Option<&PageInfo>,
    ) -> Result<()> {
        log_to_file_indexer("index_bookmark_with_page_info: creating document...");
        let doc = self.create_document(bookmark, content, page_info)?;
        log_to_file_indexer("index_bookmark_with_page_info: document created, adding to writer...");
        writer.add_document(doc)?;
        log_to_file_indexer("index_bookmark_with_page_info: document added to writer");
        Ok(())
    }

    /// Create a tantivy document from a bookmark
    pub fn create_document(
        &self,
        bookmark: &FlatBookmark,
        content: Option<&str>,
        page_info: Option<&PageInfo>,
    ) -> Result<TantivyDocument> {
        log_to_file_indexer("create_document: START");
        let domain = extract_domain(&bookmark.url).unwrap_or_default();

        let date_added = parse_date(&bookmark.date_added).unwrap_or(0);
        let date_modified = parse_date(&bookmark.date_modified).unwrap_or(0);

        log_to_file_indexer("create_document: creating TantivyDocument");
        let mut doc = TantivyDocument::new();
        doc.add_text(self.schema.id, &bookmark.id);
        doc.add_text(self.schema.url, &bookmark.url);
        doc.add_text(self.schema.title, &bookmark.name);

        if let Some(content_text) = content {
            log_to_file_indexer(&format!(
                "create_document: adding content ({} chars, {} bytes)",
                content_text.chars().count(),
                content_text.len()
            ));
            doc.add_text(self.schema.content, content_text);
            log_to_file_indexer("create_document: content added");
        }

        let folder_path = bookmark.folder_path.join("/");
        doc.add_text(self.schema.folder_path, &folder_path);
        doc.add_text(self.schema.domain, &domain);
        doc.add_i64(self.schema.date_added, date_added);
        doc.add_i64(self.schema.date_modified, date_modified);

        // Add page information if available (for PDFs)
        if let Some(page_info) = page_info {
            log_to_file_indexer(&format!(
                "create_document: adding page_info ({} pages)",
                page_info.page_count
            ));
            doc.add_u64(self.schema.page_count, page_info.page_count as u64);
            doc.add_text(self.schema.content_type, &page_info.content_type);

            // Serialize page offsets as JSON bytes
            let offsets_json = serde_json::to_vec(&page_info.page_offsets)?;
            doc.add_bytes(self.schema.page_offsets, &offsets_json);
            log_to_file_indexer("create_document: page_info added");
        } else {
            // Add default values for non-PDF content
            doc.add_u64(self.schema.page_count, 0);
            doc.add_text(self.schema.content_type, "html");
        }

        log_to_file_indexer("create_document: DONE");
        Ok(doc)
    }

    /// Build or rebuild the entire index
    pub fn build_index(&self, bookmarks: &[FlatBookmark]) -> Result<()> {
        debug!("Building index for {} bookmarks", bookmarks.len());

        let mut writer = self.create_writer(DEFAULT_WRITER_HEAP_SIZE)?;

        // Clear existing documents
        writer.delete_all_documents()?;

        // Index each bookmark
        let mut success_count = 0;
        let mut error_count = 0;

        for bookmark in bookmarks {
            match self.index_bookmark(&mut writer, bookmark, None) {
                Ok(_) => success_count += 1,
                Err(e) => {
                    warn!("Failed to index bookmark {}: {}", bookmark.id, e);
                    error_count += 1;
                }
            }
        }

        writer.commit().context("Failed to commit index")?;

        if error_count > 0 {
            warn!(
                "Index built with errors: {} successful, {} errors",
                success_count, error_count
            );
        } else {
            debug!("Index built successfully: {} documents", success_count);
        }

        Ok(())
    }

    /// Update a single bookmark in the index
    pub fn update_bookmark(&self, bookmark: &FlatBookmark, content: Option<&str>) -> Result<()> {
        self.update_bookmark_with_page_info(bookmark, content, None)
    }

    /// Update a single bookmark in the index with page information
    pub fn update_bookmark_with_page_info(
        &self,
        bookmark: &FlatBookmark,
        content: Option<&str>,
        page_info: Option<&PageInfo>,
    ) -> Result<()> {
        let mut writer = self.create_writer(10_000_000)?;

        // Delete old document
        let id_term = tantivy::Term::from_field_text(self.schema.id, &bookmark.id);
        writer.delete_term(id_term);

        // Add updated document
        self.index_bookmark_with_page_info(&mut writer, bookmark, content, page_info)?;

        writer.commit()?;
        debug!("Updated bookmark {} in index", bookmark.id);

        Ok(())
    }

    /// Delete a bookmark from the index
    pub fn delete_bookmark(&self, bookmark_id: &str) -> Result<()> {
        let mut writer = self.create_writer(10_000_000)?;

        let id_term = tantivy::Term::from_field_text(self.schema.id, bookmark_id);
        writer.delete_term(id_term);

        writer.commit()?;
        debug!("Deleted bookmark {} from index", bookmark_id);

        Ok(())
    }

    /// Delete all parts of a bookmark (for page-based indexing)
    ///
    /// This deletes the main document and up to 1000 potential parts.
    /// Note: `delete_term` doesn't report whether the term existed, so we return
    /// the number of deletion attempts (1 main + 1000 parts = 1001 total).
    /// The actual number of deleted documents may be less if fewer parts existed.
    pub fn delete_bookmark_parts(&self, bookmark_id: &str) -> Result<u32> {
        let mut writer = self.create_writer(10_000_000)?;
        let mut deletion_attempts = 0u32;

        // Delete main document
        let id_term = tantivy::Term::from_field_text(self.schema.id, bookmark_id);
        writer.delete_term(id_term);
        deletion_attempts += 1;

        // Delete all parts (up to 1000 parts max)
        for part_num in 0..1000 {
            let part_id = format!("{bookmark_id}_part_{part_num}");
            let part_term = tantivy::Term::from_field_text(self.schema.id, &part_id);
            writer.delete_term(part_term);
            deletion_attempts += 1;
        }

        writer.commit()?;
        debug!(
            "Deleted bookmark {} and its parts from index (attempted {} deletions)",
            bookmark_id, deletion_attempts
        );

        Ok(deletion_attempts)
    }

    /// Index a bookmark with page-based content splitting
    /// This splits large content into multiple documents, each containing a subset of pages
    /// Returns the number of documents created
    pub fn index_bookmark_with_page_splitting(
        &self,
        writer: &mut IndexWriter,
        bookmark: &FlatBookmark,
        content: &str,
        page_info: &PageInfo,
        max_chars_per_doc: usize,
    ) -> Result<usize> {
        log_to_file_indexer(&format!(
            "index_bookmark_with_page_splitting: START - {} pages, {} total chars, max {} per doc",
            page_info.page_count, page_info.total_chars, max_chars_per_doc
        ));

        // If content fits in a single document, use regular indexing
        if content.chars().count() <= max_chars_per_doc {
            log_to_file_indexer("index_bookmark_with_page_splitting: content fits in single doc");
            self.index_bookmark_with_page_info(writer, bookmark, Some(content), Some(page_info))?;
            return Ok(1);
        }

        // Split content by pages
        let page_offsets = &page_info.page_offsets;
        let content_chars: Vec<char> = content.chars().collect();
        let total_chars = content_chars.len();

        let mut part_num = 0;
        let mut current_start_page = 0;
        let mut current_start_char = 0;

        while current_start_char < total_chars && current_start_page < page_info.page_count {
            // Find how many pages fit in this part
            let mut end_page = current_start_page;
            let mut end_char = current_start_char;

            for page_idx in current_start_page..page_info.page_count {
                // Calculate end position for this page
                let page_end = if page_idx + 1 < page_offsets.len() {
                    page_offsets[page_idx + 1]
                } else {
                    total_chars
                };

                // Check if adding this page would exceed the limit
                let chars_in_part = page_end - current_start_char;
                if chars_in_part > max_chars_per_doc && page_idx > current_start_page {
                    // Don't include this page, stop here
                    break;
                }

                end_page = page_idx + 1;
                end_char = page_end;

                // If this single page already exceeds limit, still include it
                if chars_in_part >= max_chars_per_doc {
                    break;
                }
            }

            // Extract content for this part
            let part_content: String = content_chars[current_start_char..end_char].iter().collect();
            let part_pages = end_page - current_start_page;

            log_to_file_indexer(&format!(
                "index_bookmark_with_page_splitting: part {} - pages {}-{}, chars {}-{} ({} chars)",
                part_num,
                current_start_page + 1,
                end_page,
                current_start_char,
                end_char,
                part_content.chars().count()
            ));

            // Create page info for this part
            let part_page_info = PageInfo {
                page_count: part_pages,
                page_offsets: (0..part_pages)
                    .map(|i| {
                        if current_start_page + i < page_offsets.len() {
                            page_offsets[current_start_page + i] - current_start_char
                        } else {
                            0
                        }
                    })
                    .collect(),
                content_type: page_info.content_type.clone(),
                total_chars: part_content.chars().count(),
            };

            // Create part bookmark with modified ID
            let mut part_bookmark = bookmark.clone();
            if part_num > 0 {
                part_bookmark.id = format!("{}_part_{}", bookmark.id, part_num);
            }
            // Add page range info to title for searchability
            let page_range_suffix = if part_pages == 1 {
                format!(" [Page {}]", current_start_page + 1)
            } else {
                format!(" [Pages {}-{}]", current_start_page + 1, end_page)
            };
            part_bookmark.name = format!("{}{}", bookmark.name, page_range_suffix);

            // Index this part
            self.index_bookmark_with_page_info(
                writer,
                &part_bookmark,
                Some(&part_content),
                Some(&part_page_info),
            )?;

            part_num += 1;
            current_start_page = end_page;
            current_start_char = end_char;
        }

        log_to_file_indexer(&format!(
            "index_bookmark_with_page_splitting: DONE - created {part_num} documents"
        ));

        Ok(part_num)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tantivy::directory::MmapDirectory;
    use tempfile::TempDir;

    fn create_test_index() -> (Index, BookmarkSchema, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let schema = BookmarkSchema::new();
        let dir = MmapDirectory::open(temp_dir.path()).unwrap();
        let index = Index::create(dir, schema.schema.clone(), Default::default()).unwrap();

        // Register Lindera tokenizer for tests
        use lindera::dictionary::{DictionaryKind, load_dictionary_from_kind};
        use lindera::mode::{Mode, Penalty};
        use lindera::segmenter::Segmenter;
        use lindera_tantivy::tokenizer::LinderaTokenizer;

        let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC).unwrap();
        let mode = Mode::Decompose(Penalty::default());
        let segmenter = Segmenter::new(mode, dictionary, None);
        let tokenizer = LinderaTokenizer::from_segmenter(segmenter);
        index.tokenizers().register("lang_ja", tokenizer);

        (index, schema, temp_dir)
    }

    fn create_test_bookmark() -> FlatBookmark {
        FlatBookmark {
            id: "test-1".to_string(),
            name: "Test Bookmark".to_string(),
            url: "https://example.com/test".to_string(),
            date_added: Some("1234567890000".to_string()),
            date_modified: None,
            folder_path: vec!["Bookmarks Bar".to_string(), "Tech".to_string()],
        }
    }

    #[test]
    fn test_create_document() {
        let (_index, schema, _temp) = create_test_index();
        let indexer = BookmarkIndexer::new(_index, schema.clone());
        let bookmark = create_test_bookmark();

        let doc = indexer
            .create_document(&bookmark, Some("test content"), None)
            .unwrap();

        // Verify document has all required fields
        assert!(doc.get_first(schema.id).is_some());
        assert!(doc.get_first(schema.url).is_some());
        assert!(doc.get_first(schema.title).is_some());
        assert!(doc.get_first(schema.content).is_some());
    }

    #[test]
    fn test_index_bookmark() {
        let (index, schema, _temp) = create_test_index();
        let indexer = BookmarkIndexer::new(index, schema);
        let bookmark = create_test_bookmark();

        let mut writer = indexer.create_writer(10_000_000).unwrap();
        indexer
            .index_bookmark(&mut writer, &bookmark, None)
            .unwrap();
        writer.commit().unwrap();
    }

    #[test]
    fn test_extract_domain() {
        assert_eq!(
            extract_domain("https://example.com/path"),
            Some("example.com".to_string())
        );
        assert_eq!(
            extract_domain("http://sub.example.com"),
            Some("sub.example.com".to_string())
        );
        assert_eq!(extract_domain("invalid-url"), None);
    }

    #[test]
    fn test_parse_date() {
        assert_eq!(
            parse_date(&Some("1234567890".to_string())),
            Some(1234567890)
        );
        assert_eq!(parse_date(&Some("invalid".to_string())), None);
        assert_eq!(parse_date(&None), None);
    }

    #[test]
    fn test_index_bookmark_with_page_info() {
        let (index, schema, _temp) = create_test_index();
        let indexer = BookmarkIndexer::new(index, schema);
        let bookmark = create_test_bookmark();

        let page_info = PageInfo {
            page_count: 3,
            page_offsets: vec![0, 100, 200],
            content_type: "pdf".to_string(),
            total_chars: 300,
        };

        let mut writer = indexer.create_writer(10_000_000).unwrap();
        indexer
            .index_bookmark_with_page_info(
                &mut writer,
                &bookmark,
                Some("test content"),
                Some(&page_info),
            )
            .unwrap();
        writer.commit().unwrap();
    }

    #[test]
    fn test_page_splitting_small_content() {
        // Small content should not be split even with page info
        let (index, schema, _temp) = create_test_index();
        let indexer = BookmarkIndexer::new(index, schema);
        let bookmark = create_test_bookmark();

        let content = "Small content that fits in one document";
        let page_info = PageInfo {
            page_count: 2,
            page_offsets: vec![0, 20],
            content_type: "pdf".to_string(),
            total_chars: content.chars().count(),
        };

        let mut writer = indexer.create_writer(10_000_000).unwrap();
        let doc_count = indexer
            .index_bookmark_with_page_splitting(&mut writer, &bookmark, content, &page_info, 1000)
            .unwrap();
        writer.commit().unwrap();

        // Should create only 1 document since content is small
        assert_eq!(doc_count, 1);
    }

    #[test]
    fn test_page_splitting_large_content() {
        // Large content should be split into multiple documents
        let (index, schema, _temp) = create_test_index();
        let indexer = BookmarkIndexer::new(index, schema);
        let bookmark = create_test_bookmark();

        // Create content with 3 "pages" of 50 chars each = 150 chars total
        let page1 = "A".repeat(50);
        let page2 = "B".repeat(50);
        let page3 = "C".repeat(50);
        let content = format!("{page1}{page2}{page3}");

        let page_info = PageInfo {
            page_count: 3,
            page_offsets: vec![0, 50, 100],
            content_type: "pdf".to_string(),
            total_chars: 150,
        };

        let mut writer = indexer.create_writer(10_000_000).unwrap();
        // Set max_chars_per_doc to 60, so each page becomes its own document
        let doc_count = indexer
            .index_bookmark_with_page_splitting(&mut writer, &bookmark, &content, &page_info, 60)
            .unwrap();
        writer.commit().unwrap();

        // Should create 3 documents (one per page since each page is 50 chars and limit is 60)
        assert_eq!(doc_count, 3);
    }

    #[test]
    fn test_page_splitting_combines_small_pages() {
        // Multiple small pages should be combined when they fit
        let (index, schema, _temp) = create_test_index();
        let indexer = BookmarkIndexer::new(index, schema);
        let bookmark = create_test_bookmark();

        // Create content with 4 "pages" of 25 chars each = 100 chars total
        let page1 = "A".repeat(25);
        let page2 = "B".repeat(25);
        let page3 = "C".repeat(25);
        let page4 = "D".repeat(25);
        let content = format!("{page1}{page2}{page3}{page4}");

        let page_info = PageInfo {
            page_count: 4,
            page_offsets: vec![0, 25, 50, 75],
            content_type: "pdf".to_string(),
            total_chars: 100,
        };

        let mut writer = indexer.create_writer(10_000_000).unwrap();
        // Set max_chars_per_doc to 60, so 2 pages fit in each document
        let doc_count = indexer
            .index_bookmark_with_page_splitting(&mut writer, &bookmark, &content, &page_info, 60)
            .unwrap();
        writer.commit().unwrap();

        // Should create 2 documents (pages 1-2 and pages 3-4)
        assert_eq!(doc_count, 2);
    }

    #[test]
    fn test_delete_bookmark_parts() {
        let (index, schema, _temp) = create_test_index();
        let indexer = BookmarkIndexer::new(index, schema);
        let bookmark = create_test_bookmark();

        // First, index some content using a scoped writer
        {
            let mut writer = indexer.create_writer(10_000_000).unwrap();
            indexer
                .index_bookmark(&mut writer, &bookmark, Some("test content"))
                .unwrap();
            writer.commit().unwrap();
            // writer is dropped here, releasing the lock
        }

        // Now delete (this creates its own writer)
        let deleted = indexer.delete_bookmark_parts(&bookmark.id).unwrap();
        assert!(deleted >= 1);
    }
}
