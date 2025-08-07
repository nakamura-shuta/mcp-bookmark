use anyhow::{Context, Result};
use tantivy::{Index, IndexWriter, TantivyDocument};
use tracing::{debug, info, warn};

use super::schema::BookmarkSchema;
use crate::bookmark::FlatBookmark;

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

    /// Create an index writer
    pub fn create_writer(&self, heap_size: usize) -> Result<IndexWriter> {
        // Ensure minimum heap size for tantivy 0.24
        let min_heap = 15_000_000;
        let actual_heap = heap_size.max(min_heap);
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
        let doc = self.create_document(bookmark, content)?;
        writer.add_document(doc)?;
        Ok(())
    }

    /// Create a tantivy document from a bookmark
    pub fn create_document(
        &self,
        bookmark: &FlatBookmark,
        content: Option<&str>,
    ) -> Result<TantivyDocument> {
        let domain = extract_domain(&bookmark.url).unwrap_or_default();

        let date_added = parse_date(&bookmark.date_added).unwrap_or(0);
        let date_modified = parse_date(&bookmark.date_modified).unwrap_or(0);

        let mut doc = TantivyDocument::new();
        doc.add_text(self.schema.id, &bookmark.id);
        doc.add_text(self.schema.url, &bookmark.url);
        doc.add_text(self.schema.title, &bookmark.name);

        if let Some(content_text) = content {
            doc.add_text(self.schema.content, content_text);
        }

        let folder_path = bookmark.folder_path.join("/");
        doc.add_text(self.schema.folder_path, &folder_path);
        doc.add_text(self.schema.domain, &domain);
        doc.add_i64(self.schema.date_added, date_added);
        doc.add_i64(self.schema.date_modified, date_modified);

        Ok(doc)
    }

    /// Build or rebuild the entire index
    pub fn build_index(&self, bookmarks: &[FlatBookmark]) -> Result<()> {
        info!("Building index for {} bookmarks", bookmarks.len());

        let mut writer = self.create_writer(50_000_000)?;

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

        info!(
            "Index built: {} successful, {} errors",
            success_count, error_count
        );

        Ok(())
    }

    /// Update a single bookmark in the index
    pub fn update_bookmark(&self, bookmark: &FlatBookmark, content: Option<&str>) -> Result<()> {
        let mut writer = self.create_writer(10_000_000)?;

        // Delete old document
        let id_term = tantivy::Term::from_field_text(self.schema.id, &bookmark.id);
        writer.delete_term(id_term);

        // Add updated document
        self.index_bookmark(&mut writer, bookmark, content)?;

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
}

/// Extract domain from URL
fn extract_domain(url: &str) -> Option<String> {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
}

/// Parse date string to timestamp
fn parse_date(date: &Option<String>) -> Option<i64> {
    date.as_ref()?.parse::<i64>().ok()
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
            .create_document(&bookmark, Some("test content"))
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
}
