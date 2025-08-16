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
    profile_id: String,
    folder_name: String,
}

impl NativeMessagingHost {
    fn new() -> Self {
        Self {
            indexer: None,
            profile_id: "Extension".to_string(),
            folder_name: "Bookmarks".to_string(),
        }
    }

    fn init_tantivy(&mut self) -> Result<()> {
        // Use the same directory as MCP server with proper naming
        let index_key = format!(
            "{}_{}",
            self.profile_id.replace(['/', ' '], "_"),
            self.folder_name.replace(['/', ' '], "_")
        );
        let index_path = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("mcp-bookmark")
            .join(index_key);

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
            "Tantivy index initialized for folder: {}",
            self.folder_name
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
                        "folder_name": self.folder_name
                    }
                })
            }

            "set_context" => {
                let params = &message["params"];
                if let Some(folder) = params["folder_name"].as_str() {
                    self.folder_name = folder.to_string();
                    if let Some(profile) = params["profile_id"].as_str() {
                        self.profile_id = profile.to_string();
                    }
                    self.indexer = None; // Clear existing indexer to force re-init
                    log_to_file(&format!(
                        "Context set - Profile: {}, Folder: {}",
                        self.profile_id, self.folder_name
                    ));
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "status": "ok",
                            "profile_id": self.profile_id,
                            "folder_name": self.folder_name
                        }
                    })
                } else {
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32602,
                            "message": "Invalid params: folder_name required"
                        }
                    })
                }
            }

            // Keep old method for compatibility
            "set_folder" => {
                if let Some(folder) = message["params"]["folder_name"].as_str() {
                    self.folder_name = folder.to_string();
                    self.indexer = None;
                    log_to_file(&format!("Folder set to: {}", self.folder_name));
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "status": "ok",
                            "folder_name": self.folder_name
                        }
                    })
                } else {
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32602,
                            "message": "Invalid params: folder_name required"
                        }
                    })
                }
            }

            "index_bookmark" => {
                // Extract folder name from params if provided
                if let Some(folder) = message["params"]["folder_name"].as_str() {
                    self.profile_id = "Extension".to_string(); // Always use Extension
                    self.folder_name = folder.to_string();
                    self.indexer = None; // Reset indexer to use new folder
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
