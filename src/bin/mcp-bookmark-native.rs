use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::path::PathBuf;

// Import Tantivy integration from main crate
use mcp_bookmark::bookmark::FlatBookmark;
use mcp_bookmark::search::BookmarkSchema;
use mcp_bookmark::search::indexer::BookmarkIndexer;
use tantivy::Index;

// Import Lindera tokenizer
use lindera::dictionary::{DictionaryKind, load_dictionary_from_kind};
use lindera::mode::{Mode, Penalty};
use lindera::segmenter::Segmenter;
use lindera_tantivy::tokenizer::LinderaTokenizer;

fn log_to_file(msg: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/mcp-bookmark-native.log")
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

struct NativeMessagingHost {
    indexer: Option<BookmarkIndexer>,
    index_name: String,
    metadata: Option<IndexMetadata>,
}

impl NativeMessagingHost {
    fn new() -> Self {
        Self {
            indexer: None,
            index_name: "Extension_Bookmarks".to_string(),
            metadata: None,
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
            log_to_file(&format!("Loaded metadata with {} bookmarks", 
                self.metadata.as_ref().map(|m| m.bookmarks.len()).unwrap_or(0)));
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
            log_to_file(&format!("Saved metadata with {} bookmarks", metadata.bookmarks.len()));
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
                // Update index name if provided in params
                if let Some(index_name) = message["params"]["index_name"].as_str() {
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
                self.index_bookmark(message["params"].clone(), id)
            }

            "get_stats" => self.get_index_stats(id),

            "list_indexes" => self.list_indexes(id),
            
            "sync_bookmarks" => self.sync_bookmarks(message["params"].clone(), id),
            
            "check_for_updates" => self.check_for_updates(message["params"].clone(), id),

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
        
        // Check if we should skip this bookmark
        if skip_if_unchanged {
            if let Some(metadata) = &self.metadata {
                if let Some(existing) = metadata.bookmarks.get(&bookmark.id) {
                    let content_hash = Self::calculate_content_hash(content);
                    if existing.date_modified == bookmark.date_modified 
                        && existing.content_hash == Some(content_hash) {
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
            "Indexing bookmark: {} with content: {} chars",
            bookmark.url,
            content.map(|c| c.len()).unwrap_or(0)
        ));

        // Index the bookmark
        match self.index_single_bookmark(indexer, &bookmark, content) {
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
                        }
                    );
                    
                    // Save metadata periodically (every 10 bookmarks)
                    if metadata.bookmarks.len() % 10 == 0 {
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

    fn index_single_bookmark(
        &self,
        indexer: &BookmarkIndexer,
        bookmark: &FlatBookmark,
        content: Option<&str>,
    ) -> Result<()> {
        // Use update_bookmark which handles deletion of old document
        indexer.update_bookmark(bookmark, content)?;
        Ok(())
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
            log_to_file(&format!("Failed to save metadata: {}", e));
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

    // Write message length (4 bytes, little-endian)
    let len = json_bytes.len() as u32;
    io::stdout().write_all(&len.to_le_bytes())?;

    // Write message
    io::stdout().write_all(json_bytes)?;
    io::stdout().flush()?;

    Ok(())
}
