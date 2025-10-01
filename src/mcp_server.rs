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
use crate::config::Config;
use crate::search::{SearchParams, search_manager_trait::SearchManagerTrait};

// Tool request/response types
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FullTextSearchRequest {
    #[schemars(
        description = "Search query to find within indexed page contents extracted from bookmarked websites"
    )]
    pub query: String,
    #[schemars(description = "Filter results to specific bookmark folder (optional)")]
    pub folder: Option<String>,
    #[schemars(description = "Filter results to specific domain (e.g., 'github.com') (optional)")]
    pub domain: Option<String>,
    #[schemars(description = "Maximum number of search results to return (default: 20)")]
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetBookmarkContentRequest {
    #[schemars(
        description = "Exact URL of the bookmark to retrieve full indexed page content from the local Tantivy search index"
    )]
    pub url: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetBookmarkContentRangeRequest {
    #[schemars(description = "Exact URL of the PDF bookmark")]
    pub url: String,
    #[schemars(
        description = "Start page number (1-indexed, inclusive). For single page, set start_page = end_page"
    )]
    pub start_page: usize,
    #[schemars(
        description = "End page number (1-indexed, inclusive). For single page, set start_page = end_page"
    )]
    pub end_page: usize,
}

#[derive(Debug, Clone)]
pub struct BookmarkServer {
    #[allow(dead_code)]
    pub reader: Arc<BookmarkReader>,
    pub search_manager: Arc<dyn SearchManagerTrait>,
    pub config: Config,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl BookmarkServer {
    pub fn new(reader: Arc<BookmarkReader>, search_manager: Arc<dyn SearchManagerTrait>) -> Self {
        Self {
            reader,
            search_manager,
            config: Config::default(),
            tool_router: Self::tool_router(),
        }
    }

    fn _create_resource(&self, uri: &str, name: &str, description: &str) -> Resource {
        let mut resource = RawResource::new(uri, name.to_string());
        resource.description = Some(description.to_string());
        resource.mime_type = Some("application/json".to_string());
        resource.no_annotation()
    }

    #[tool(
        description = "Search through indexed webpage contents extracted from bookmarked sites using Tantivy full-text search engine"
    )]
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
            Ok(mut results) => {
                // Include indexing status
                let status = self.search_manager.get_indexing_status();
                let is_complete = self.search_manager.is_indexing_complete();

                // Limit response size for MCP to avoid token limits
                let max_snippet_length = self.config.max_snippet_length;
                for result in &mut results {
                    // Limit snippet text (UTF-8 safe)
                    if result.snippet.len() > max_snippet_length {
                        let mut end = max_snippet_length;
                        while end > 0 && !result.snippet.is_char_boundary(end) {
                            end -= 1;
                        }
                        result.snippet.truncate(end);
                        if !result.snippet.ends_with("...") {
                            result.snippet.push_str("...");
                        }
                    }
                }

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

    #[tool(
        description = "Get the current status of the bookmark content indexing process and check if indexing is complete"
    )]
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

    #[tool(
        description = "Retrieve complete indexed webpage content for a specific bookmark URL from the local Tantivy search index. For large PDF files, consider using get_bookmark_content_range instead to retrieve specific pages."
    )]
    async fn get_bookmark_content(
        &self,
        Parameters(req): Parameters<GetBookmarkContentRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Get content from URL (from index or new fetch)
        match self.search_manager.get_content_by_url(&req.url).await {
            Ok(Some(content)) => {
                // Check content size and warn if too large
                const WARNING_THRESHOLD: usize = 100_000; // 100k characters
                let size_warning = if content.len() > WARNING_THRESHOLD {
                    Some(format!(
                        "⚠️ Large content detected ({} chars). For better performance with large PDFs, consider using get_bookmark_content_range to retrieve specific pages instead of the entire document.",
                        content.len()
                    ))
                } else {
                    None
                };

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

                let mut response = json!({
                    "url": req.url,
                    "title": title,
                    "folder_path": folder_path,
                    "content": content,
                    "content_length": content.len(),
                });

                if let Some(warning) = size_warning {
                    response["warning"] = json!(warning);
                }

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

    #[tool(
        description = "Retrieve specific page(s) from a PDF bookmark. For single page, set start_page = end_page. For range, set start_page < end_page. Page numbers are 1-indexed."
    )]
    async fn get_bookmark_content_range(
        &self,
        Parameters(req): Parameters<GetBookmarkContentRangeRequest>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .search_manager
            .get_page_range_content(&req.url, req.start_page, req.end_page)
            .await
        {
            Ok(Some(content)) => {
                let page_desc = if req.start_page == req.end_page {
                    format!("page {}", req.start_page)
                } else {
                    format!("pages {}-{}", req.start_page, req.end_page)
                };

                let response = json!({
                    "url": req.url,
                    "start_page": req.start_page,
                    "end_page": req.end_page,
                    "page_range": page_desc,
                    "content": content,
                    "content_length": content.len(),
                });

                let content_json = serde_json::to_string_pretty(&response)
                    .unwrap_or_else(|e| format!("Error serializing response: {e}"));
                Ok(CallToolResult::success(vec![Content::text(content_json)]))
            }
            Ok(None) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Content not found for URL: {}. The bookmark may not exist in the index.",
                req.url
            ))])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error retrieving pages {}-{} for URL {}: {}",
                req.start_page, req.end_page, req.url, e
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
            instructions: Some("Chrome bookmark MCP server provides access to indexed content from your Chrome bookmarks. Use 'search_bookmarks_fulltext' to search within indexed webpage contents (including titles and URLs), and 'get_bookmark_content' to retrieve full indexed content for specific URLs. All content is pre-indexed locally using Tantivy search engine via Chrome extension.".to_string()),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let resources = vec![
            // Resource: bookmark://tree
            self._create_resource(
                "bookmark://tree",
                "Bookmark Tree",
                "Full Chrome bookmark tree",
            ),
        ];

        // Folder resources not available with INDEX_NAME approach
        // All bookmarks are accessed through search tools

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
            // Tree view not available with INDEX_NAME approach
            // Use search tools to access bookmarks
            Err(McpError::resource_not_found(
                "Bookmark tree is not available when using INDEX_NAME. Use search tools instead."
                    .to_string(),
                Some(json!({ "uri": uri })),
            ))
        } else if uri.starts_with("bookmark://folder/") {
            // Folder resources not available with INDEX_NAME approach
            Err(McpError::resource_not_found(
                "Folder resources are not available when using INDEX_NAME. Use search tools instead.".to_string(),
                Some(json!({ "uri": uri })),
            ))
        } else {
            Err(McpError::resource_not_found(
                format!("Unknown resource: {uri}"),
                Some(json!({ "uri": uri })),
            ))
        }
    }
}
