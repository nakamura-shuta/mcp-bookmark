use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::time::Instant;

// Import Tantivy integration from main crate
use mcp_bookmark::bookmark::FlatBookmark;
use mcp_bookmark::search::indexer::{BookmarkIndexer, PageInfo};
use mcp_bookmark::search::schema::BookmarkSchema;
use tantivy::Index;

// Import Lindera tokenizer
use lindera::dictionary::{DictionaryKind, load_dictionary_from_kind};
use lindera::mode::{Mode, Penalty};
use lindera::segmenter::Segmenter;
use lindera_tantivy::tokenizer::LinderaTokenizer;

// Configuration constants
const LOG_FILE_PATH: &str = "/tmp/mcp-bookmark-native.log";
const INDEX_WRITER_HEAP_SIZE: usize = 50_000_000;

fn log_to_file(msg: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_FILE_PATH)
    {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let _ = writeln!(file, "[{timestamp}] {msg}");
    }
}

// Metadata for tracking indexed bookmarks
#[derive(Debug, Serialize, Deserialize, Clone)]
struct BookmarkMetadata {
    url: String,
    date_modified: Option<String>,
    indexed_at: u64,
    content_hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct IndexMetadata {
    bookmarks: HashMap<String, BookmarkMetadata>, // key: bookmark ID
    last_full_sync: u64,
}

// Batch processing state
#[derive(Debug)]
struct BatchState {
    #[allow(dead_code)] // Used for logging and debugging
    batch_id: String,
    total: usize,
    received: HashSet<usize>,
    bookmarks: Vec<(FlatBookmark, Option<String>)>,
    start_time: Instant,
}

// Chunk session state for large content transfer
#[derive(Debug)]
struct ChunkSession {
    bookmark_id: String,
    total_chunks: usize,
    received_chunks: HashMap<usize, String>, // chunk_index -> chunk_content
    // Bookmark metadata (received with first chunk)
    url: Option<String>,
    title: Option<String>,
    folder_path: Option<Vec<String>>,
    date_added: Option<String>,
    date_modified: Option<String>,
    page_info: Option<PageInfo>,
    start_time: Instant,
}

struct NativeMessagingHost {
    indexer: Option<BookmarkIndexer>,
    index_name: String,
    metadata: Option<IndexMetadata>,
    batches: HashMap<String, BatchState>,
    chunk_sessions: HashMap<String, ChunkSession>, // chunk_session_id -> ChunkSession
}

impl NativeMessagingHost {
    fn new() -> Self {
        Self {
            indexer: None,
            index_name: "Extension_Bookmarks".to_string(),
            metadata: None,
            batches: HashMap::new(),
            chunk_sessions: HashMap::new(),
        }
    }

    fn metadata_path(&self) -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("mcp-bookmark")
            .join(&self.index_name)
            .join("index_metadata.json")
    }

    fn load_metadata(&mut self) -> Result<()> {
        let path = self.metadata_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            self.metadata = Some(serde_json::from_str(&content)?);
            log_to_file(&format!(
                "Loaded metadata with {} bookmarks",
                self.metadata
                    .as_ref()
                    .map(|m| m.bookmarks.len())
                    .unwrap_or(0)
            ));
        } else {
            self.metadata = Some(IndexMetadata {
                bookmarks: HashMap::new(),
                last_full_sync: 0,
            });
            log_to_file("Created new metadata");
        }
        Ok(())
    }

    fn save_metadata(&self) -> Result<()> {
        if let Some(metadata) = &self.metadata {
            let path = self.metadata_path();
            std::fs::create_dir_all(path.parent().unwrap())?;
            let content = serde_json::to_string_pretty(metadata)?;
            std::fs::write(&path, content)?;
            log_to_file(&format!(
                "Saved metadata with {} bookmarks",
                metadata.bookmarks.len()
            ));
        }
        Ok(())
    }

    fn calculate_content_hash(content: Option<&str>) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.unwrap_or("").hash(&mut hasher);
        hasher.finish().to_string()
    }

    fn init_tantivy(&mut self) -> Result<()> {
        // Use the same directory as MCP server with index name
        let index_path = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("mcp-bookmark")
            .join(&self.index_name);

        // Create directory if it doesn't exist
        std::fs::create_dir_all(&index_path)?;

        // Create schema
        let schema = BookmarkSchema::new();

        // Open or create index
        let index = if index_path.join("meta.json").exists() {
            Index::open_in_dir(&index_path)?
        } else {
            Index::create_in_dir(&index_path, schema.schema.clone())?
        };

        // Register Lindera tokenizer for Japanese text processing
        Self::register_lindera_tokenizer(&index)?;

        self.indexer = Some(BookmarkIndexer::new(index, schema));

        // Load metadata after initializing indexer
        self.load_metadata()?;

        log_to_file(&format!(
            "Tantivy index initialized with Lindera tokenizer: {}",
            self.index_name
        ));
        Ok(())
    }

    /// Register Lindera tokenizer for Japanese text
    fn register_lindera_tokenizer(index: &Index) -> Result<()> {
        log_to_file("Registering Lindera tokenizer for Japanese text processing");

        // Load IPADIC dictionary
        let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC)
            .context("Failed to load IPADIC dictionary")?;

        // Use Decompose mode for better search results
        let mode = Mode::Decompose(Penalty::default());
        let user_dictionary = None;

        // Create Segmenter with the dictionary
        let segmenter = Segmenter::new(mode, dictionary, user_dictionary);

        // Create Lindera tokenizer from segmenter
        let tokenizer = LinderaTokenizer::from_segmenter(segmenter);

        // Register the tokenizer with name "lang_ja"
        index.tokenizers().register("lang_ja", tokenizer);

        log_to_file("Lindera tokenizer registered successfully");
        Ok(())
    }

    fn handle_message(&mut self, message: Value) -> Value {
        let method = message["method"].as_str().unwrap_or("");
        let id = message["id"].clone();
        log_to_file(&format!("handle_message: method={method}"));

        match method {
            "ping" => {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "status": "ok",
                        "tantivy_initialized": self.indexer.is_some(),
                        "index_name": self.index_name
                    }
                })
            }

            "index_bookmark" => {
                log_to_file("handle_message: index_bookmark branch");
                // Update index name if provided in params
                if let Some(index_name) = message["params"]["index_name"].as_str() {
                    self.index_name = index_name.to_string();
                    self.indexer = None; // Reset indexer to use new index
                    log_to_file(&format!("Index name updated to: {}", self.index_name));
                }
                log_to_file("handle_message: before init_tantivy check");

                // Initialize indexer if needed
                if self.indexer.is_none() {
                    log_to_file("handle_message: calling init_tantivy...");
                    if let Err(e) = self.init_tantivy() {
                        return json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": {
                                "code": -32603,
                                "message": format!("Failed to initialize index: {}", e)
                            }
                        });
                    }
                    log_to_file("handle_message: init_tantivy completed");
                }
                log_to_file("handle_message: calling index_bookmark...");
                let result = self.index_bookmark(message["params"].clone(), id);
                log_to_file("handle_message: index_bookmark completed");
                result
            }

            "get_stats" => self.get_index_stats(id),

            "list_indexes" => self.list_indexes(id),

            "sync_bookmarks" => self.sync_bookmarks(message["params"].clone(), id),

            "check_for_updates" => self.check_for_updates(message["params"].clone(), id),

            // Chunk-based content transfer for large PDFs
            "index_bookmark_chunk" => self.index_bookmark_chunk(message["params"].clone(), id),

            // Batch processing methods
            "batch_start" => self.batch_start(message["params"].clone(), id),
            "batch_add" => self.batch_add(message["params"].clone(), id),
            "batch_end" => self.batch_end(message["params"].clone(), id),
            "progress" => self.handle_progress(message["params"].clone(), id),

            // Single-message batch indexing (optimized)
            "index_bookmarks_batch" => self.index_bookmarks_batch(message["params"].clone(), id),

            // Legacy MCP methods for compatibility
            "initialize" => {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "0.1.0",
                        "capabilities": {
                            "tools": {}
                        },
                        "serverInfo": {
                            "name": "mcp-bookmark-native",
                            "version": "0.2.0"
                        }
                    }
                })
            }

            _ => {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32601,
                        "message": format!("Method not found: {}", method)
                    }
                })
            }
        }
    }

    fn index_bookmark(&mut self, params: Value, id: Value) -> Value {
        log_to_file("index_bookmark: START");

        // Update index name if provided in params
        if let Some(index_name) = params["index_name"].as_str() {
            if self.index_name != index_name {
                self.index_name = index_name.to_string();
                self.indexer = None; // Reset indexer to use new index
                log_to_file(&format!("Index name updated to: {}", self.index_name));
            }
        }
        log_to_file("index_bookmark: After index name check");

        // Initialize indexer if needed
        if self.indexer.is_none() {
            log_to_file("index_bookmark: Initializing Tantivy...");
            if let Err(e) = self.init_tantivy() {
                return json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32603,
                        "message": format!("Failed to initialize index: {}", e)
                    }
                });
            }
            log_to_file("index_bookmark: Tantivy initialized");
        }

        let Some(indexer) = &self.indexer else {
            return json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32603,
                    "message": "Tantivy index not initialized"
                }
            });
        };

        // Parse bookmark data
        let bookmark = FlatBookmark {
            id: params["id"].as_str().unwrap_or("").to_string(),
            name: params["title"].as_str().unwrap_or("").to_string(),
            url: params["url"].as_str().unwrap_or("").to_string(),
            folder_path: params["folder_path"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default(),
            date_added: params["date_added"].as_str().map(String::from),
            date_modified: params["date_modified"].as_str().map(String::from),
        };

        let content = params["content"].as_str();
        let skip_if_unchanged = params["skip_if_unchanged"].as_bool().unwrap_or(false);

        // Parse page_info if available (for PDFs)
        let page_info = params["page_info"].as_object().and_then(|obj| {
            let page_count = obj.get("page_count")?.as_u64()? as usize;
            let page_offsets = obj
                .get("page_offsets")?
                .as_array()?
                .iter()
                .filter_map(|v| v.as_u64().map(|n| n as usize))
                .collect::<Vec<_>>();
            let content_type = obj.get("content_type")?.as_str()?.to_string();
            let total_chars = obj.get("total_chars")?.as_u64()? as usize;

            Some(PageInfo {
                page_count,
                page_offsets,
                content_type,
                total_chars,
            })
        });

        // Check if we should skip this bookmark
        if skip_if_unchanged {
            if let Some(metadata) = &self.metadata {
                if let Some(existing) = metadata.bookmarks.get(&bookmark.id) {
                    let content_hash = Self::calculate_content_hash(content);
                    if existing.date_modified == bookmark.date_modified
                        && existing.content_hash == Some(content_hash)
                    {
                        log_to_file(&format!("Skipping unchanged bookmark: {}", bookmark.url));
                        return json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "status": "skipped",
                                "url": bookmark.url
                            }
                        });
                    }
                }
            }
        }

        log_to_file(&format!(
            "Indexing bookmark: {} with content: {} chars, page_info: {}",
            bookmark.url,
            content.map(|c| c.len()).unwrap_or(0),
            page_info.is_some()
        ));

        // Index the bookmark with page info if available
        match self.index_single_bookmark_with_page_info(
            indexer,
            &bookmark,
            content,
            page_info.as_ref(),
        ) {
            Ok(_) => {
                // Update metadata
                if let Some(metadata) = &mut self.metadata {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();

                    metadata.bookmarks.insert(
                        bookmark.id.clone(),
                        BookmarkMetadata {
                            url: bookmark.url.clone(),
                            date_modified: bookmark.date_modified.clone(),
                            indexed_at: now,
                            content_hash: Some(Self::calculate_content_hash(content)),
                        },
                    );

                    // Save metadata periodically (every 10 bookmarks) or always for small collections
                    if metadata.bookmarks.len() % 10 == 0 || metadata.bookmarks.len() <= 5 {
                        let _ = self.save_metadata();
                    }
                }

                log_to_file(&format!("Successfully indexed bookmark: {}", bookmark.url));
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "status": "indexed",
                        "url": bookmark.url
                    }
                })
            }
            Err(e) => {
                log_to_file(&format!("Failed to index bookmark: {e}"));
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32603,
                        "message": format!("Failed to index: {}", e)
                    }
                })
            }
        }
    }

    fn index_single_bookmark_with_page_info(
        &self,
        indexer: &BookmarkIndexer,
        bookmark: &FlatBookmark,
        content: Option<&str>,
        page_info: Option<&PageInfo>,
    ) -> Result<()> {
        log_to_file("index_single_bookmark_with_page_info: START");

        // Max chars per document to prevent Lindera tokenizer from hanging
        // 100K chars is a safe limit for Japanese text tokenization
        // (~300KB in UTF-8, tokenizable in reasonable time)
        const MAX_CHARS_PER_DOC: usize = 100_000;

        // Create a writer for this single bookmark
        log_to_file("index_single_bookmark_with_page_info: creating writer...");
        let mut writer = indexer.create_writer(INDEX_WRITER_HEAP_SIZE)?;
        log_to_file("index_single_bookmark_with_page_info: writer created");

        // Delete any existing parts of this bookmark first
        // Use 0..1000 to match delete_bookmark_parts (supports up to 1000 parts)
        let id_term = tantivy::Term::from_field_text(indexer.schema().id, &bookmark.id);
        writer.delete_term(id_term);
        // Delete potential parts (up to 1000 parts max, matching indexer.rs)
        for part_num in 0..1000 {
            let part_id = format!("{}_part_{}", bookmark.id, part_num);
            let part_term = tantivy::Term::from_field_text(indexer.schema().id, &part_id);
            writer.delete_term(part_term);
        }
        log_to_file("index_single_bookmark_with_page_info: existing documents deleted");

        // Index with page-based splitting if we have page info and large content
        if let (Some(content_str), Some(pi)) = (content, page_info) {
            let char_count = content_str.chars().count();
            log_to_file(&format!(
                "index_single_bookmark_with_page_info: content has {} chars, {} pages",
                char_count, pi.page_count
            ));

            if char_count > MAX_CHARS_PER_DOC && pi.page_count > 1 {
                // Use page-based splitting for large PDFs
                log_to_file("index_single_bookmark_with_page_info: using page-based splitting");
                let doc_count = indexer.index_bookmark_with_page_splitting(
                    &mut writer,
                    bookmark,
                    content_str,
                    pi,
                    MAX_CHARS_PER_DOC,
                )?;
                log_to_file(&format!(
                    "index_single_bookmark_with_page_info: created {doc_count} documents via page splitting"
                ));
            } else {
                // Small content or single page - use regular indexing
                log_to_file(&format!(
                    "index_single_bookmark_with_page_info: indexing with page_info ({} pages)",
                    pi.page_count
                ));
                indexer.index_bookmark_with_page_info(
                    &mut writer,
                    bookmark,
                    Some(content_str),
                    Some(pi),
                )?;
                log_to_file(
                    "index_single_bookmark_with_page_info: index_bookmark_with_page_info completed",
                );
            }
        } else if let Some(pi) = page_info {
            // No content but have page info
            log_to_file(&format!(
                "index_single_bookmark_with_page_info: indexing with page_info ({} pages), no content",
                pi.page_count
            ));
            indexer.index_bookmark_with_page_info(&mut writer, bookmark, content, Some(pi))?;
            log_to_file(
                "index_single_bookmark_with_page_info: index_bookmark_with_page_info completed",
            );
        } else {
            // No page info - regular indexing
            log_to_file("index_single_bookmark_with_page_info: indexing without page_info");
            indexer.index_bookmark(&mut writer, bookmark, content)?;
            log_to_file("index_single_bookmark_with_page_info: index_bookmark completed");
        }

        // Commit
        log_to_file("index_single_bookmark_with_page_info: committing...");
        writer.commit()?;
        log_to_file("index_single_bookmark_with_page_info: commit completed");
        Ok(())
    }

    /// Handle chunk-based content transfer for large PDFs
    /// Chunks are received one at a time and reassembled when all chunks are received
    fn index_bookmark_chunk(&mut self, params: Value, id: Value) -> Value {
        // Update index name if provided
        if let Some(index_name) = params["index_name"].as_str() {
            if self.index_name != index_name {
                self.index_name = index_name.to_string();
                self.indexer = None; // Reset indexer to use new index
                log_to_file(&format!(
                    "Chunk: Index name updated to: {}",
                    self.index_name
                ));
            }
        }

        // Initialize indexer if needed
        if self.indexer.is_none() {
            if let Err(e) = self.init_tantivy() {
                return json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32603,
                        "message": format!("Failed to initialize index: {}", e)
                    }
                });
            }
        }

        // Parse chunk parameters
        let chunk_session_id = params["chunk_session_id"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let bookmark_id = params["bookmark_id"].as_str().unwrap_or("").to_string();
        let chunk_index = params["chunk_index"].as_u64().unwrap_or(0) as usize;
        let total_chunks = params["total_chunks"].as_u64().unwrap_or(1) as usize;
        let is_last_chunk = params["is_last_chunk"].as_bool().unwrap_or(false);
        let chunk_content = params["chunk_content"].as_str().unwrap_or("").to_string();

        log_to_file(&format!(
            "Chunk received: session={}, index={}/{}, is_last={}, content_len={}",
            chunk_session_id,
            chunk_index + 1,
            total_chunks,
            is_last_chunk,
            chunk_content.len()
        ));

        // Get or create chunk session
        let session = self
            .chunk_sessions
            .entry(chunk_session_id.clone())
            .or_insert_with(|| ChunkSession {
                bookmark_id: bookmark_id.clone(),
                total_chunks,
                received_chunks: HashMap::new(),
                url: None,
                title: None,
                folder_path: None,
                date_added: None,
                date_modified: None,
                page_info: None,
                start_time: Instant::now(),
            });

        // Store chunk content
        session.received_chunks.insert(chunk_index, chunk_content);

        // Parse bookmark metadata from first chunk (chunk_index == 0)
        if chunk_index == 0 {
            session.url = params["url"].as_str().map(String::from);
            session.title = params["title"].as_str().map(String::from);
            session.folder_path = params["folder_path"].as_array().map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect()
            });
            session.date_added = params["date_added"].as_str().map(String::from);
            session.date_modified = params["date_modified"].as_str().map(String::from);

            // Parse page_info if available
            session.page_info = params["page_info"].as_object().and_then(|obj| {
                let page_count = obj.get("page_count")?.as_u64()? as usize;
                let page_offsets = obj
                    .get("page_offsets")?
                    .as_array()?
                    .iter()
                    .filter_map(|v| v.as_u64().map(|n| n as usize))
                    .collect::<Vec<_>>();
                let content_type = obj.get("content_type")?.as_str()?.to_string();
                let total_chars = obj.get("total_chars")?.as_u64()? as usize;

                Some(PageInfo {
                    page_count,
                    page_offsets,
                    content_type,
                    total_chars,
                })
            });

            log_to_file(&format!(
                "Chunk session metadata: url={:?}, title={:?}, page_info={:?}",
                session.url,
                session.title,
                session.page_info.is_some()
            ));
        }

        // Check if all chunks received
        if session.received_chunks.len() == session.total_chunks {
            log_to_file(&format!(
                "All {} chunks received for session {}, reassembling content...",
                session.total_chunks, chunk_session_id
            ));

            // Reassemble content in order
            let mut full_content = String::new();
            for i in 0..session.total_chunks {
                if let Some(chunk) = session.received_chunks.get(&i) {
                    full_content.push_str(chunk);
                } else {
                    log_to_file(&format!("Missing chunk {i} in session {chunk_session_id}"));
                    return json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32603,
                            "message": format!("Missing chunk {i} in session")
                        }
                    });
                }
            }

            log_to_file(&format!(
                "Reassembled content: {} chars from {} chunks",
                full_content.len(),
                session.total_chunks
            ));

            // Create bookmark from session data
            let bookmark = FlatBookmark {
                id: session.bookmark_id.clone(),
                name: session.title.clone().unwrap_or_default(),
                url: session.url.clone().unwrap_or_default(),
                folder_path: session.folder_path.clone().unwrap_or_default(),
                date_added: session.date_added.clone(),
                date_modified: session.date_modified.clone(),
            };

            let page_info = session.page_info.clone();
            let duration = session.start_time.elapsed();

            // Remove session before indexing
            self.chunk_sessions.remove(&chunk_session_id);

            // Index the reassembled bookmark
            let Some(indexer) = &self.indexer else {
                return json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32603,
                        "message": "Tantivy index not initialized"
                    }
                });
            };

            match self.index_single_bookmark_with_page_info(
                indexer,
                &bookmark,
                Some(&full_content),
                page_info.as_ref(),
            ) {
                Ok(_) => {
                    // Update metadata
                    if let Some(metadata) = &mut self.metadata {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs();

                        metadata.bookmarks.insert(
                            bookmark.id.clone(),
                            BookmarkMetadata {
                                url: bookmark.url.clone(),
                                date_modified: bookmark.date_modified.clone(),
                                indexed_at: now,
                                content_hash: Some(Self::calculate_content_hash(Some(
                                    &full_content,
                                ))),
                            },
                        );
                        let _ = self.save_metadata();
                    }

                    log_to_file(&format!(
                        "Successfully indexed chunked bookmark: {} ({} chars, {} chunks, {:?})",
                        bookmark.url,
                        full_content.len(),
                        total_chunks,
                        duration
                    ));

                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "status": "indexed",
                            "url": bookmark.url,
                            "content_length": full_content.len(),
                            "chunks_received": total_chunks,
                            "duration_ms": duration.as_millis() as u64
                        }
                    })
                }
                Err(e) => {
                    log_to_file(&format!("Failed to index chunked bookmark: {e}"));
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32603,
                            "message": format!("Failed to index: {e}")
                        }
                    })
                }
            }
        } else {
            // More chunks expected, return acknowledgment
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "status": "chunk_received",
                    "chunk_index": chunk_index,
                    "chunks_received": session.received_chunks.len(),
                    "total_chunks": session.total_chunks
                }
            })
        }
    }

    fn get_index_stats(&self, id: Value) -> Value {
        let Some(_indexer) = &self.indexer else {
            return json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32603,
                    "message": "Tantivy index not initialized"
                }
            });
        };

        // TODO: Implement actual stats gathering
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "status": "ok",
                "indexed": true
            }
        })
    }

    fn list_indexes(&self, id: Value) -> Value {
        let base_path = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("mcp-bookmark");

        let mut indexes = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&base_path) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_dir() {
                        let path = entry.path();
                        let name = entry.file_name().to_string_lossy().to_string();

                        // Check if it's a valid index by looking for meta.json
                        if path.join("meta.json").exists() {
                            // Calculate size
                            let size = Self::calculate_dir_size(&path).unwrap_or(0);

                            // Count documents (simplified - just check if index can be opened)
                            let doc_count = if let Ok(index) = Index::open_in_dir(&path) {
                                // Register Lindera tokenizer for the opened index
                                let _ = Self::register_lindera_tokenizer(&index);

                                index
                                    .reader()
                                    .ok()
                                    .map(|reader| reader.searcher().num_docs() as usize)
                                    .unwrap_or(0)
                            } else {
                                0
                            };

                            indexes.push(json!({
                                "name": name,
                                "size": size,
                                "doc_count": doc_count
                            }));
                        }
                    }
                }
            }
        }

        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "indexes": indexes
            }
        })
    }

    fn calculate_dir_size(path: &std::path::Path) -> Result<u64> {
        let mut size = 0;
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        size += metadata.len();
                    } else if metadata.is_dir() {
                        size += Self::calculate_dir_size(&entry.path()).unwrap_or(0);
                    }
                }
            }
        }
        Ok(size)
    }

    fn check_for_updates(&mut self, params: Value, id: Value) -> Value {
        // Initialize if needed
        if self.indexer.is_none() {
            if let Err(e) = self.init_tantivy() {
                return json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32603,
                        "message": format!("Failed to initialize index: {}", e)
                    }
                });
            }
        }

        let bookmarks = params["bookmarks"].as_array();
        if bookmarks.is_none() {
            return json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32602,
                    "message": "Missing bookmarks parameter"
                }
            });
        }

        let mut updates_needed = Vec::new();
        let mut new_bookmarks = Vec::new();

        if let Some(metadata) = &self.metadata {
            for bookmark in bookmarks.unwrap() {
                let id_str = bookmark["id"].as_str().unwrap_or("");
                let date_modified = bookmark["date_modified"].as_str();

                if let Some(existing) = metadata.bookmarks.get(id_str) {
                    // Check if bookmark has been modified
                    if existing.date_modified != date_modified.map(String::from) {
                        updates_needed.push(id_str.to_string());
                    }
                } else {
                    // New bookmark
                    new_bookmarks.push(id_str.to_string());
                }
            }
        } else {
            // No metadata, all bookmarks are new
            for bookmark in bookmarks.unwrap() {
                if let Some(id_str) = bookmark["id"].as_str() {
                    new_bookmarks.push(id_str.to_string());
                }
            }
        }

        log_to_file(&format!(
            "Check for updates: {} new, {} updated",
            new_bookmarks.len(),
            updates_needed.len()
        ));

        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "new_bookmarks": new_bookmarks,
                "updated_bookmarks": updates_needed,
                "total_indexed": self.metadata.as_ref().map(|m| m.bookmarks.len()).unwrap_or(0)
            }
        })
    }

    fn sync_bookmarks(&mut self, _params: Value, id: Value) -> Value {
        // Save metadata after sync
        if let Err(e) = self.save_metadata() {
            log_to_file(&format!("Failed to save metadata: {e}"));
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if let Some(metadata) = &mut self.metadata {
            metadata.last_full_sync = now;
        }

        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "status": "synced",
                "bookmark_count": self.metadata.as_ref().map(|m| m.bookmarks.len()).unwrap_or(0)
            }
        })
    }

    // Batch processing methods
    fn batch_start(&mut self, params: Value, id: Value) -> Value {
        let batch_id = params["batch_id"].as_str().unwrap_or("").to_string();
        let total = params["total"].as_u64().unwrap_or(0) as usize;

        // Update index name if provided
        if let Some(index_name) = params["index_name"].as_str() {
            self.index_name = index_name.to_string();
            log_to_file(&format!("Batch using index: {}", self.index_name));
        }

        if batch_id.is_empty() || total == 0 {
            return json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32602,
                    "message": "Invalid batch parameters"
                }
            });
        }

        // Initialize indexer if needed
        if self.indexer.is_none() {
            if let Err(e) = self.init_tantivy() {
                return json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32603,
                        "message": format!("Failed to initialize index: {}", e)
                    }
                });
            }
        }

        // Create batch state
        let batch = BatchState {
            batch_id: batch_id.clone(),
            total,
            received: HashSet::new(),
            bookmarks: Vec::with_capacity(total),
            start_time: Instant::now(),
        };

        self.batches.insert(batch_id.clone(), batch);
        log_to_file(&format!("Started batch {batch_id} with {total} bookmarks"));

        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "status": "started",
                "batch_id": batch_id
            }
        })
    }

    fn batch_add(&mut self, params: Value, id: Value) -> Value {
        let batch_id = params["batch_id"].as_str().unwrap_or("").to_string();
        let index = params["index"].as_u64().unwrap_or(0) as usize;

        // Parse bookmark
        let bookmark = FlatBookmark {
            id: params["bookmark"]["id"].as_str().unwrap_or("").to_string(),
            name: params["bookmark"]["name"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            url: params["bookmark"]["url"].as_str().unwrap_or("").to_string(),
            folder_path: params["bookmark"]["folder_path"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default(),
            date_added: params["bookmark"]["date_added"].as_str().map(String::from),
            date_modified: params["bookmark"]["date_modified"]
                .as_str()
                .map(String::from),
        };

        let content = params["content"].as_str().map(String::from);

        // Add to batch
        if let Some(batch) = self.batches.get_mut(&batch_id) {
            if batch.received.contains(&index) {
                log_to_file(&format!("Duplicate index {index} in batch {batch_id}"));
                return json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {"status": "duplicate"}
                });
            }

            batch.received.insert(index);
            batch.bookmarks.push((bookmark, content));

            // Check if we should commit (buffer full or complete)
            let should_commit = batch.bookmarks.len() >= 50 || batch.received.len() == batch.total;
            let received_count = batch.received.len();
            let total_count = batch.total;

            if should_commit {
                let batch_id_clone = batch_id.clone();
                // Mutable borrow is automatically released here
                self.commit_batch_bookmarks(&batch_id_clone);
            }

            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "status": "added",
                    "received": received_count,
                    "total": total_count
                }
            })
        } else {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32602,
                    "message": format!("Batch {} not found", batch_id)
                }
            })
        }
    }

    fn batch_end(&mut self, params: Value, id: Value) -> Value {
        let batch_id = params["batch_id"].as_str().unwrap_or("").to_string();

        if let Some(mut batch) = self.batches.remove(&batch_id) {
            // Commit remaining bookmarks
            if !batch.bookmarks.is_empty() {
                self.commit_batch_bookmarks_internal(&mut batch);
            }

            let duration = batch.start_time.elapsed();
            let total = batch.total;
            let received = batch.received.len();

            log_to_file(&format!(
                "Batch {batch_id} completed: {received}/{total} bookmarks in {duration:?}"
            ));

            // Save metadata
            if let Err(e) = self.save_metadata() {
                log_to_file(&format!("Failed to save metadata: {e}"));
            }

            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "batch_id": batch_id,
                    "success_count": received,
                    "failed_count": total - received,
                    "duration_ms": duration.as_millis() as u64
                }
            })
        } else {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32602,
                    "message": format!("Batch {} not found", batch_id)
                }
            })
        }
    }

    fn handle_progress(&mut self, params: Value, id: Value) -> Value {
        let batch_id = params["batch_id"].as_str().unwrap_or("");
        let completed = params["completed"].as_u64().unwrap_or(0);
        let total = params["total"].as_u64().unwrap_or(0);

        log_to_file(&format!(
            "Progress: {completed}/{total} for batch {batch_id}"
        ));

        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {"status": "acknowledged"}
        })
    }

    // Single-message batch indexing - receives all bookmarks with content
    fn index_bookmarks_batch(&mut self, params: Value, id: Value) -> Value {
        // Update index name if provided
        if let Some(index_name) = params["index_name"].as_str() {
            self.index_name = index_name.to_string();
            self.indexer = None; // Reset indexer to use new index
            log_to_file(&format!("Index name updated to: {}", self.index_name));
        }

        // Initialize indexer if needed
        if self.indexer.is_none() {
            if let Err(e) = self.init_tantivy() {
                return json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32603,
                        "message": format!("Failed to initialize index: {}", e)
                    }
                });
            }
        }

        let Some(indexer) = &self.indexer else {
            return json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32603,
                    "message": "Tantivy index not initialized"
                }
            });
        };

        // Parse bookmarks array
        let bookmarks_json = params["bookmarks"].as_array();
        if bookmarks_json.is_none() {
            return json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32602,
                    "message": "Missing or invalid bookmarks array"
                }
            });
        }

        let bookmarks_json = bookmarks_json.unwrap();
        let total = bookmarks_json.len();

        log_to_file(&format!(
            "Received batch of {} bookmarks for index {}",
            total, self.index_name
        ));

        // Create a single writer for all bookmarks
        let mut writer = match indexer.create_writer(50_000_000) {
            Ok(w) => w,
            Err(e) => {
                return json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32603,
                        "message": format!("Failed to create writer: {}", e)
                    }
                });
            }
        };

        let mut success_count = 0;
        let mut error_count = 0;

        // Process all bookmarks
        for bookmark_json in bookmarks_json {
            // Parse bookmark
            let bookmark = FlatBookmark {
                id: bookmark_json["id"].as_str().unwrap_or("").to_string(),
                name: bookmark_json["title"].as_str().unwrap_or("").to_string(),
                url: bookmark_json["url"].as_str().unwrap_or("").to_string(),
                folder_path: bookmark_json["folder_path"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default(),
                date_added: bookmark_json["date_added"].as_str().map(String::from),
                date_modified: bookmark_json["date_modified"].as_str().map(String::from),
            };

            let content = bookmark_json["content"].as_str().map(String::from);

            // Parse page_info if available (for PDFs)
            let page_info = bookmark_json["page_info"].as_object().and_then(|obj| {
                let page_count = obj.get("page_count")?.as_u64()? as usize;
                let page_offsets = obj
                    .get("page_offsets")?
                    .as_array()?
                    .iter()
                    .filter_map(|v| v.as_u64().map(|n| n as usize))
                    .collect::<Vec<_>>();
                let content_type = obj.get("content_type")?.as_str()?.to_string();
                let total_chars = obj.get("total_chars")?.as_u64()? as usize;

                Some(PageInfo {
                    page_count,
                    page_offsets,
                    content_type,
                    total_chars,
                })
            });

            // Index the bookmark with content and page info from extension
            let index_result = if let Some(ref page_info) = page_info {
                indexer.index_bookmark_with_page_info(
                    &mut writer,
                    &bookmark,
                    content.as_deref(),
                    Some(page_info),
                )
            } else {
                indexer.index_bookmark(&mut writer, &bookmark, content.as_deref())
            };

            if let Err(e) = index_result {
                log_to_file(&format!("Failed to index {}: {}", bookmark.url, e));
                error_count += 1;
            } else {
                success_count += 1;

                // Update metadata
                if let Some(metadata) = &mut self.metadata {
                    metadata.bookmarks.insert(
                        bookmark.id.clone(),
                        BookmarkMetadata {
                            url: bookmark.url.clone(),
                            date_modified: bookmark.date_modified.clone(),
                            indexed_at: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                            content_hash: content
                                .as_ref()
                                .map(|c| Self::calculate_content_hash(Some(c))),
                        },
                    );
                }
            }
        }

        // Commit all at once
        if let Err(e) = writer.commit() {
            log_to_file(&format!("Failed to commit: {e}"));
            return json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32603,
                    "message": format!("Failed to commit index: {}", e)
                }
            });
        }

        // Save metadata
        if let Err(e) = self.save_metadata() {
            log_to_file(&format!("Failed to save metadata: {e}"));
        }

        log_to_file(&format!(
            "Successfully indexed {success_count}/{total} bookmarks"
        ));

        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "success_count": success_count,
                "error_count": error_count,
                "total": total
            }
        })
    }

    fn commit_batch_bookmarks(&mut self, batch_id: &str) {
        if let Some(mut batch) = self.batches.remove(batch_id) {
            self.commit_batch_bookmarks_internal(&mut batch);
            // Re-insert if not complete
            if !batch.bookmarks.is_empty() {
                self.batches.insert(batch_id.to_string(), batch);
            }
        }
    }

    fn commit_batch_bookmarks_internal(&mut self, batch: &mut BatchState) {
        if let Some(indexer) = &self.indexer {
            let count = batch.bookmarks.len();
            log_to_file(&format!("Committing {count} bookmarks from batch"));

            // Create writer
            if let Ok(mut writer) = indexer.create_writer(INDEX_WRITER_HEAP_SIZE) {
                for (bookmark, content) in batch.bookmarks.drain(..) {
                    // Index the bookmark with the content from extension
                    if let Err(e) =
                        indexer.index_bookmark(&mut writer, &bookmark, content.as_deref())
                    {
                        log_to_file(&format!("Failed to index bookmark: {e}"));
                    } else {
                        // Update metadata
                        if let Some(metadata) = &mut self.metadata {
                            let content_hash = content
                                .as_ref()
                                .map(|c| Self::calculate_content_hash(Some(c)));
                            metadata.bookmarks.insert(
                                bookmark.id.clone(),
                                BookmarkMetadata {
                                    url: bookmark.url,
                                    date_modified: bookmark.date_modified,
                                    indexed_at: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs(),
                                    content_hash,
                                },
                            );
                        }
                    }
                }

                if let Err(e) = writer.commit() {
                    log_to_file(&format!("Failed to commit: {e}"));
                } else {
                    log_to_file(&format!("Successfully committed {count} bookmarks"));
                }
            }
        }
    }
}

fn main() -> io::Result<()> {
    log_to_file("Native messaging host started");

    let mut host = NativeMessagingHost::new();

    loop {
        // Read message length (4 bytes, little-endian)
        let mut len_bytes = [0u8; 4];
        match io::stdin().read_exact(&mut len_bytes) {
            Ok(_) => {}
            Err(e) => {
                log_to_file(&format!("Error reading length bytes: {e}"));
                break; // EOF or error, exit
            }
        }

        let msg_len = u32::from_le_bytes(len_bytes) as usize;
        log_to_file(&format!("Received message length: {msg_len}"));

        if msg_len == 0 || msg_len > 100_000_000 {
            // Increased from 10MB to 100MB
            log_to_file(&format!("Invalid message length: {msg_len}"));
            continue;
        }

        // Read message
        let mut buffer = vec![0u8; msg_len];
        match io::stdin().read_exact(&mut buffer) {
            Ok(_) => {}
            Err(e) => {
                log_to_file(&format!("Error reading message: {e}"));
                return Err(e);
            }
        }

        log_to_file(&format!(
            "Received message: {:?}",
            String::from_utf8_lossy(&buffer)
        ));

        // Parse JSON
        let message: Value = match serde_json::from_slice(&buffer) {
            Ok(msg) => msg,
            Err(e) => {
                log_to_file(&format!("Failed to parse JSON: {e}"));
                continue;
            }
        };

        // Handle the message
        let response = host.handle_message(message.clone());
        log_to_file(&format!(
            "Sending response for method: {:?}",
            message["method"]
        ));

        // Send response
        send_response(response)?;
    }

    Ok(())
}

fn send_response(response: Value) -> io::Result<()> {
    let json_str = response.to_string();
    let json_bytes = json_str.as_bytes();

    // Log response size for debugging
    log_to_file(&format!(
        "Response size: {} bytes ({:.2} KB)",
        json_bytes.len(),
        json_bytes.len() as f64 / 1024.0
    ));

    // Check for 1MB limit (Native→Chrome direction)
    const MAX_RESPONSE_SIZE: usize = 1024 * 1024; // 1MB
    if json_bytes.len() > MAX_RESPONSE_SIZE {
        log_to_file(&format!(
            "WARNING: Response exceeds 1MB limit! Size: {} bytes",
            json_bytes.len()
        ));
    }

    // Write message length (4 bytes, little-endian)
    let len = json_bytes.len() as u32;
    io::stdout().write_all(&len.to_le_bytes())?;

    // Write message
    io::stdout().write_all(json_bytes)?;
    io::stdout().flush()?;

    Ok(())
}
