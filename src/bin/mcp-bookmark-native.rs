use anyhow::Result;
use serde_json::{Value, json};
use std::fs::OpenOptions;
use std::io::{self, Read, Write};

// Import Tantivy integration from main crate
use mcp_bookmark::bookmark::FlatBookmark;
use mcp_bookmark::search::{BookmarkIndexer, BookmarkSchema};
use tantivy::Index;

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

struct NativeMessagingHost {
    indexer: Option<BookmarkIndexer>,
    index_name: String,
}

impl NativeMessagingHost {
    fn new() -> Self {
        Self {
            indexer: None,
            index_name: "Extension_Bookmarks".to_string(),
        }
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

        self.indexer = Some(BookmarkIndexer::new(index, schema));
        log_to_file(&format!(
            "Tantivy index initialized: {}",
            self.index_name
        ));
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

            "clear_index" => self.clear_index(id),

            "get_stats" => self.get_index_stats(id),
            
            "list_indexes" => self.list_indexes(id),

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

        log_to_file(&format!(
            "Indexing bookmark: {} with content: {} chars",
            bookmark.url,
            content.map(|c| c.len()).unwrap_or(0)
        ));

        // Index the bookmark
        match self.index_single_bookmark(indexer, &bookmark, content) {
            Ok(_) => {
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

    fn clear_index(&mut self, id: Value) -> Value {
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

        match indexer.create_writer(15_000_000) {
            Ok(mut writer) => {
                writer.delete_all_documents().ok();
                match writer.commit() {
                    Ok(_) => {
                        log_to_file("Index cleared successfully");
                        json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "status": "cleared"
                            }
                        })
                    }
                    Err(e) => {
                        log_to_file(&format!("Failed to clear index: {e}"));
                        json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": {
                                "code": -32603,
                                "message": format!("Failed to clear index: {}", e)
                            }
                        })
                    }
                }
            }
            Err(e) => {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32603,
                        "message": format!("Failed to create writer: {}", e)
                    }
                })
            }
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
                                index.reader().ok()
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
