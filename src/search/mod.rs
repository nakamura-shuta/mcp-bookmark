// Module declarations
pub mod common;
pub mod indexer;
pub mod multi_index;
pub mod query_parser;
pub mod schema;
pub mod scored_snippet;
pub mod search_manager;
pub mod search_manager_trait;
pub mod tokenizer;
pub mod unified_searcher;

// Re-export public APIs
pub use indexer::PageInfo;
pub use multi_index::MultiIndexSearchManager;
pub use search_manager::SearchManager;
pub use unified_searcher::{SearchParams, SearchResult};
