use tantivy::schema::{
    FAST, Field, IndexRecordOption, STORED, STRING, Schema, TextFieldIndexing, TextOptions,
};

use super::tokenizer::JAPANESE_TOKENIZER_NAME;

/// Bookmark index schema definition
#[derive(Clone, Debug)]
pub struct BookmarkSchema {
    pub schema: Schema,
    pub id: Field,
    pub url: Field,
    pub title: Field,
    pub content: Field,
    pub folder_path: Field,
    pub domain: Field,
    pub date_added: Field,
    pub date_modified: Field,
    // Page information fields (for chunked content retrieval)
    pub page_count: Field,
    pub page_offsets: Field,
    pub content_type: Field,
}

impl BookmarkSchema {
    /// Create a new bookmark schema
    pub fn new() -> Self {
        let mut builder = Schema::builder();

        // Unique identifier (stored, not indexed for exact retrieval)
        let id = builder.add_text_field("id", STRING | STORED);

        // URL field (stored as string for exact match)
        let url = builder.add_text_field("url", STRING | STORED);

        // Configure text options with Lindera tokenizer for Japanese text
        let text_field_indexing = TextFieldIndexing::default()
            .set_tokenizer(JAPANESE_TOKENIZER_NAME) // Use Lindera tokenizer
            .set_index_option(IndexRecordOption::WithFreqsAndPositions);

        let text_options = TextOptions::default()
            .set_indexing_options(text_field_indexing)
            .set_stored();

        // Title field (stored and indexed with Lindera tokenizer)
        let title = builder.add_text_field("title", text_options.clone());

        // Content field (indexed and stored for full-text search with Lindera tokenizer)
        let content = builder.add_text_field("content", text_options);

        // Folder path for filtering (stored as string)
        let folder_path = builder.add_text_field("folder_path", STRING | STORED);

        // Domain for filtering (with fast field for efficient filtering)
        let domain = builder.add_text_field("domain", STRING | STORED | FAST);

        // Dates for time-based filtering (fast fields for range queries)
        let date_added = builder.add_i64_field("date_added", STORED | FAST);
        let date_modified = builder.add_i64_field("date_modified", STORED | FAST);

        // Page information fields for chunked content retrieval (optional, only for PDFs)
        let page_count = builder.add_u64_field("page_count", STORED | FAST);
        let page_offsets = builder.add_bytes_field("page_offsets", STORED);
        let content_type = builder.add_text_field("content_type", STRING | STORED);

        let schema = builder.build();

        Self {
            schema,
            id,
            url,
            title,
            content,
            folder_path,
            domain,
            date_added,
            date_modified,
            page_count,
            page_offsets,
            content_type,
        }
    }

    /// Get fields for text search
    pub fn text_fields(&self) -> Vec<Field> {
        // URL is now STRING field, so only search in title and content
        vec![self.title, self.content]
    }
}

impl Default for BookmarkSchema {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_creation() {
        let schema = BookmarkSchema::new();

        // Verify all fields exist
        assert!(schema.schema.get_field("id").is_ok());
        assert!(schema.schema.get_field("url").is_ok());
        assert!(schema.schema.get_field("title").is_ok());
        assert!(schema.schema.get_field("content").is_ok());
        assert!(schema.schema.get_field("folder_path").is_ok());
        assert!(schema.schema.get_field("domain").is_ok());
        assert!(schema.schema.get_field("date_added").is_ok());
        assert!(schema.schema.get_field("date_modified").is_ok());
        assert!(schema.schema.get_field("page_count").is_ok());
        assert!(schema.schema.get_field("page_offsets").is_ok());
        assert!(schema.schema.get_field("content_type").is_ok());
    }

    #[test]
    fn test_text_fields() {
        let schema = BookmarkSchema::new();
        let text_fields = schema.text_fields();

        assert_eq!(text_fields.len(), 2);
        assert!(text_fields.contains(&schema.title));
        assert!(text_fields.contains(&schema.content));
    }
}
