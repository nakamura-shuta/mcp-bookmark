use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::*,
    schemars,
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::bookmark::BookmarkReader;
use crate::search::{ContentIndexManager, SearchParams};

// Tool request/response types
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchBookmarksRequest {
    #[schemars(description = "Search query for bookmark title or URL")]
    pub query: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FullTextSearchRequest {
    #[schemars(description = "Search query for full-text search")]
    pub query: String,
    #[schemars(description = "Optional folder filter")]
    pub folder: Option<String>,
    #[schemars(description = "Optional domain filter")]
    pub domain: Option<String>,
    #[schemars(description = "Maximum number of results (default: 20)")]
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetBookmarkContentRequest {
    #[schemars(description = "URL of the bookmark to fetch content from")]
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct BookmarkServer {
    pub reader: Arc<BookmarkReader>,
    pub search_manager: Arc<ContentIndexManager>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl BookmarkServer {
    pub fn new(reader: Arc<BookmarkReader>, search_manager: Arc<ContentIndexManager>) -> Self {
        Self {
            reader,
            search_manager,
            tool_router: Self::tool_router(),
        }
    }

    fn _create_resource(&self, uri: &str, name: &str, description: &str) -> Resource {
        let mut resource = RawResource::new(uri, name.to_string());
        resource.description = Some(description.to_string());
        resource.mime_type = Some("application/json".to_string());
        resource.no_annotation()
    }

    #[tool(description = "Search bookmarks by title or URL")]
    fn search_bookmarks(
        &self,
        Parameters(req): Parameters<SearchBookmarksRequest>,
    ) -> Result<CallToolResult, McpError> {
        match self.reader.search_bookmarks(&req.query) {
            Ok(results) => {
                let content = serde_json::to_string_pretty(&results)
                    .unwrap_or_else(|e| format!("Error serializing results: {e}"));
                Ok(CallToolResult::success(vec![Content::text(content)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error searching bookmarks: {e}"
            ))])),
        }
    }

    #[tool(description = "List all bookmark folders")]
    fn list_bookmark_folders(&self) -> Result<CallToolResult, McpError> {
        match self.reader.list_filtered_folders() {
            Ok(folders) => {
                let content = serde_json::to_string_pretty(&folders)
                    .unwrap_or_else(|e| format!("Error serializing folders: {e}"));
                Ok(CallToolResult::success(vec![Content::text(content)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error listing folders: {e}"
            ))])),
        }
    }

    #[tool(description = "Full-text search through bookmarks including page content")]
    async fn search_bookmarks_fulltext(
        &self,
        Parameters(req): Parameters<FullTextSearchRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Build search parameters
        let results = if req.folder.is_some() || req.domain.is_some() {
            // Search with filters
            let mut params = SearchParams::new(&req.query);
            if let Some(folder) = req.folder {
                params = params.with_folder(folder);
            }
            if let Some(domain) = req.domain {
                params = params.with_domain(domain);
            }
            if let Some(limit) = req.limit {
                params = params.with_limit(limit);
            }
            self.search_manager.search_advanced(&params).await
        } else {
            // Normal search
            self.search_manager
                .search(&req.query, req.limit.unwrap_or(20))
                .await
        };

        match results {
            Ok(results) => {
                // Include indexing status
                let status = self.search_manager.get_indexing_status();
                let is_complete = self.search_manager.is_indexing_complete();

                let response = json!({
                    "results": results,
                    "total_results": results.len(),
                    "indexing_status": status,
                    "indexing_complete": is_complete,
                    "note": if !is_complete && results.is_empty() {
                        "No results found. Content indexing in progress - results may be incomplete."
                    } else {
                        ""
                    }
                });

                let content = serde_json::to_string_pretty(&response)
                    .unwrap_or_else(|e| format!("Error serializing results: {e}"));
                Ok(CallToolResult::success(vec![Content::text(content)]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error searching bookmarks: {e}"
            ))])),
        }
    }

    #[tool(description = "Get indexing status")]
    fn get_indexing_status(&self) -> Result<CallToolResult, McpError> {
        let status = self.search_manager.get_indexing_status();
        let is_complete = self.search_manager.is_indexing_complete();

        let response = json!({
            "status": status,
            "is_complete": is_complete,
        });

        let content =
            serde_json::to_string_pretty(&response).unwrap_or_else(|e| format!("Error: {e}"));
        Ok(CallToolResult::success(vec![Content::text(content)]))
    }

    #[tool(description = "Get list of available Chrome profiles")]
    fn get_available_profiles(&self) -> Result<CallToolResult, McpError> {
        use crate::chrome_profile::ProfileResolver;

        match ProfileResolver::new() {
            Ok(resolver) => {
                match resolver.list_all_profiles() {
                    Ok(profiles) => {
                        // Get current profile
                        let current_profile = resolver.get_current_profile();
                        let current_dir = current_profile.as_ref().map(|p| &p.directory_name);

                        let profile_list: Vec<_> = profiles
                            .iter()
                            .map(|p| {
                                json!({
                                    "display_name": p.display_name,
                                    "directory_name": p.directory_name,
                                    "bookmark_count": p.bookmark_count,
                                    "size_kb": p.size_kb,
                                    "is_current": Some(&p.directory_name) == current_dir,
                                    "path": p.path.to_string_lossy(),
                                })
                            })
                            .collect();

                        let response = json!({
                            "profiles": profile_list,
                            "total": profiles.len(),
                        });

                        let content = serde_json::to_string_pretty(&response)
                            .unwrap_or_else(|e| format!("Error: {e}"));
                        Ok(CallToolResult::success(vec![Content::text(content)]))
                    }
                    Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                        "Error listing profiles: {e}"
                    ))])),
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error initializing profile resolver: {e}"
            ))])),
        }
    }

    #[tool(description = "Get full content of a bookmark from index or fetch if needed")]
    async fn get_bookmark_content(
        &self,
        Parameters(req): Parameters<GetBookmarkContentRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Get content from URL (from index or new fetch)
        match self.search_manager.get_content_by_url(&req.url).await {
            Ok(Some(content)) => {
                // Also get bookmark information
                let search_results = self
                    .search_manager
                    .search(&req.url, 1)
                    .await
                    .unwrap_or_default();

                let (title, folder_path) = if let Some(result) = search_results.first() {
                    if result.url == req.url {
                        (result.title.clone(), Some(result.folder_path.clone()))
                    } else {
                        ("Unknown".to_string(), None)
                    }
                } else {
                    ("Unknown".to_string(), None)
                };

                let response = json!({
                    "url": req.url,
                    "title": title,
                    "folder_path": folder_path,
                    "content": content,
                    "content_length": content.len(),
                });

                let content_json = serde_json::to_string_pretty(&response)
                    .unwrap_or_else(|e| format!("Error serializing response: {e}"));
                Ok(CallToolResult::success(vec![Content::text(content_json)]))
            }
            Ok(None) => {
                // If content could not be fetched
                Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed to fetch content for URL: {}. The page may be unavailable or require authentication.",
                    req.url
                ))]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error fetching content for URL {}: {}",
                req.url, e
            ))])),
        }
    }
}

#[tool_handler]
impl ServerHandler for BookmarkServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            server_info: Implementation {
                name: "mcp-bookmark".to_string(),
                version: "0.1.0".to_string(),
            },
            instructions: Some("Chrome bookmark MCP server provides access to your Chrome bookmarks. You can search bookmarks, list folders, and fetch content from bookmark URLs.".to_string()),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let mut resources = vec![];

        // Resource: bookmark://tree
        resources.push(self._create_resource(
            "bookmark://tree",
            "Bookmark Tree",
            "Full Chrome bookmark tree",
        ));

        // Resource: bookmark://folder/{path} for each folder
        if let Ok(folders) = self.reader.list_filtered_folders() {
            for folder_path in folders {
                let uri = format!("bookmark://folder/{}", folder_path.join("/"));
                let name = folder_path.last().unwrap_or(&"Unknown".to_string()).clone();

                resources.push(self._create_resource(
                    &uri,
                    &format!("Folder: {name}"),
                    &format!("Bookmarks in {} folder", folder_path.join("/")),
                ));
            }
        }

        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        ReadResourceRequestParam { uri }: ReadResourceRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        if uri == "bookmark://tree" {
            // Return filtered bookmarks
            match self.reader.get_all_bookmarks() {
                Ok(bookmarks) => {
                    let content = serde_json::to_string_pretty(&bookmarks)
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                    Ok(ReadResourceResult {
                        contents: vec![ResourceContents::text(content, uri)],
                    })
                }
                Err(e) => Err(McpError::resource_not_found(
                    e.to_string(),
                    Some(json!({ "uri": uri })),
                )),
            }
        } else if uri.starts_with("bookmark://folder/") {
            // Return bookmarks from specific folder
            let path = uri.strip_prefix("bookmark://folder/").unwrap();
            let folder_path: Vec<String> = path.split('/').map(String::from).collect();

            match self.reader.get_folder_bookmarks(&folder_path) {
                Ok(bookmarks) => {
                    let content = serde_json::to_string_pretty(&bookmarks)
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                    Ok(ReadResourceResult {
                        contents: vec![ResourceContents::text(content, uri)],
                    })
                }
                Err(e) => Err(McpError::resource_not_found(
                    e.to_string(),
                    Some(json!({ "uri": uri })),
                )),
            }
        } else {
            Err(McpError::resource_not_found(
                format!("Unknown resource: {uri}"),
                Some(json!({ "uri": uri })),
            ))
        }
    }
}
