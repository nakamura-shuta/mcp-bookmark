// Module declarations
pub mod common;
pub mod content_index;
pub mod indexer;
pub mod multi_index;
pub mod schema;
pub mod scored_snippet;
pub mod search_manager;
pub mod search_manager_trait;
pub mod tokenizer;
pub mod unified_searcher;

// Re-export public APIs
pub use multi_index::MultiIndexSearchManager;
pub use schema::BookmarkSchema;
pub use search_manager::SearchManager;
pub use search_manager_trait::SearchManagerTrait;
pub use unified_searcher::{SearchParams, SearchResult};
