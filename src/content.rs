use anyhow::{Context, Result};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub og_title: Option<String>,
    pub og_description: Option<String>,
    pub og_image: Option<String>,
    pub og_type: Option<String>,
    pub favicon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageContent {
    pub text_content: Option<String>,
    pub main_content: Option<String>,
    pub html_title: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ContentFetcher {
    client: reqwest::Client,
}

impl ContentFetcher {
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { client })
    }

    pub async fn fetch_page(&self, url: &str) -> Result<String> {
        let response = self.client.get(url).send().await.map_err(|e| {
            // Log detailed error information
            tracing::warn!("Fetch error for {}: {:?}", url, e);
            anyhow::anyhow!("Failed to fetch URL: {} - Error: {}", url, e)
        })?;

        let status = response.status();
        if !status.is_success() {
            if status.is_redirection() {
                tracing::warn!("Unexpected redirect for {}: status {}", url, status);
            }
            anyhow::bail!("HTTP error for {}: status {}", url, status);
        }

        let html = response
            .text()
            .await
            .context("Failed to read response body")?;

        Ok(html)
    }

    pub fn extract_metadata(&self, html: &str, base_url: &str) -> PageMetadata {
        let document = Html::parse_document(html);

        let title_selector = Selector::parse("title").ok();
        let meta_selector = Selector::parse("meta").ok();
        let link_selector = Selector::parse("link[rel~='icon']").ok();

        let mut metadata = PageMetadata {
            title: None,
            description: None,
            og_title: None,
            og_description: None,
            og_image: None,
            og_type: None,
            favicon_url: None,
        };

        if let Some(selector) = title_selector {
            if let Some(element) = document.select(&selector).next() {
                metadata.title = Some(element.inner_html().trim().to_string());
            }
        }

        if let Some(selector) = meta_selector {
            for element in document.select(&selector) {
                if let Some(property) = element.value().attr("property") {
                    if let Some(content) = element.value().attr("content") {
                        match property {
                            "og:title" => metadata.og_title = Some(content.to_string()),
                            "og:description" => metadata.og_description = Some(content.to_string()),
                            "og:image" => {
                                metadata.og_image = Some(self.resolve_url(base_url, content))
                            }
                            "og:type" => metadata.og_type = Some(content.to_string()),
                            _ => {}
                        }
                    }
                }

                if let Some(name) = element.value().attr("name") {
                    if name == "description" {
                        if let Some(content) = element.value().attr("content") {
                            metadata.description = Some(content.to_string());
                        }
                    }
                }
            }
        }

        if let Some(selector) = link_selector {
            if let Some(element) = document.select(&selector).next() {
                if let Some(href) = element.value().attr("href") {
                    metadata.favicon_url = Some(self.resolve_url(base_url, href));
                }
            }
        }

        metadata
    }

    pub fn extract_content(&self, html: &str) -> PageContent {
        let document = Html::parse_document(html);

        let title_selector = Selector::parse("title").ok();
        let body_selector = Selector::parse("body").ok();
        let article_selector = Selector::parse("article, main, [role='main']").ok();

        let mut content = PageContent {
            text_content: None,
            main_content: None,
            html_title: None,
        };

        if let Some(selector) = title_selector {
            if let Some(element) = document.select(&selector).next() {
                content.html_title = Some(element.inner_html().trim().to_string());
            }
        }

        if let Some(selector) = body_selector {
            if let Some(element) = document.select(&selector).next() {
                let text = self.extract_text_from_element(element);
                content.text_content = Some(text);
            }
        }

        if let Some(selector) = article_selector {
            if let Some(element) = document.select(&selector).next() {
                let text = self.extract_text_from_element(element);
                content.main_content = Some(text);
            }
        }

        content
    }

    fn extract_text_from_element(&self, element: scraper::ElementRef) -> String {
        let mut text = String::new();
        for node in element.text() {
            let trimmed = node.trim();
            if !trimmed.is_empty() {
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(trimmed);
            }
        }
        text
    }

    fn resolve_url(&self, base_url: &str, path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") {
            return path.to_string();
        }

        if let Ok(base) = url::Url::parse(base_url) {
            if let Ok(resolved) = base.join(path) {
                return resolved.to_string();
            }
        }

        path.to_string()
    }
}
