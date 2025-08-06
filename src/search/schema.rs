use tantivy::schema::{Field, Schema, FAST, STORED, STRING, TEXT};

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
}

impl BookmarkSchema {
    /// Create a new bookmark schema
    pub fn new() -> Self {
        let mut builder = Schema::builder();
        
        // Unique identifier (stored, not indexed for exact retrieval)
        let id = builder.add_text_field("id", STRING | STORED);
        
        // URL field (stored and indexed)
        let url = builder.add_text_field("url", TEXT | STORED);
        
        // Title field (stored and indexed with higher weight)
        let title = builder.add_text_field("title", TEXT | STORED);
        
        // Content field (indexed but not stored to save space)
        let content = builder.add_text_field("content", TEXT);
        
        // Folder path for filtering (stored as string)
        let folder_path = builder.add_text_field("folder_path", STRING | STORED);
        
        // Domain for filtering (with fast field for efficient filtering)
        let domain = builder.add_text_field("domain", STRING | STORED | FAST);
        
        // Dates for time-based filtering (fast fields for range queries)
        let date_added = builder.add_i64_field("date_added", STORED | FAST);
        let date_modified = builder.add_i64_field("date_modified", STORED | FAST);
        
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
        }
    }

    /// Get fields for text search
    pub fn text_fields(&self) -> Vec<Field> {
        vec![self.title, self.url, self.content]
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
    }

    #[test]
    fn test_text_fields() {
        let schema = BookmarkSchema::new();
        let text_fields = schema.text_fields();
        
        assert_eq!(text_fields.len(), 3);
        assert!(text_fields.contains(&schema.title));
        assert!(text_fields.contains(&schema.url));
        assert!(text_fields.contains(&schema.content));
    }
}