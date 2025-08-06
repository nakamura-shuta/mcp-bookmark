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
use crate::content::ContentFetcher;

// Tool request/response types
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchBookmarksRequest {
    #[schemars(description = "Search query for bookmark title or URL")]
    pub query: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetBookmarkContentRequest {
    #[schemars(description = "URL to fetch content from")]
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct BookmarkServer {
    pub reader: Arc<BookmarkReader>,
    pub fetcher: Arc<ContentFetcher>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl BookmarkServer {
    pub fn new(reader: Arc<BookmarkReader>, fetcher: Arc<ContentFetcher>) -> Self {
        Self {
            reader,
            fetcher,
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
        match self.reader.list_all_folders() {
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

    #[tool(description = "Fetch content from a bookmark URL")]
    async fn get_bookmark_content(
        &self,
        Parameters(req): Parameters<GetBookmarkContentRequest>,
    ) -> Result<CallToolResult, McpError> {
        match self.fetcher.fetch_page(&req.url).await {
            Ok(html) => {
                let metadata = self.fetcher.extract_metadata(&html, &req.url);
                let page_content = self.fetcher.extract_content(&html);

                let content = json!({
                    "url": req.url,
                    "title": metadata.title,
                    "description": metadata.description,
                    "og_title": metadata.og_title,
                    "og_description": metadata.og_description,
                    "text_content": page_content.text_content,
                    "main_content": page_content.main_content,
                });

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&content)
                        .unwrap_or_else(|e| format!("Error serializing content: {e}")),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error fetching content: {e}"
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
                name: "chrome-bookmark-mcp".to_string(),
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
        if let Ok(folders) = self.reader.list_all_folders() {
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
            // Return full bookmark tree
            match self.reader.read() {
                Ok(tree) => {
                    let content = serde_json::to_string_pretty(&tree)
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
