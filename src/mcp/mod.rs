use crate::core::config::Config;
use crate::core::models::SearchOptions;
use crate::core::patent_search::{PatentSearch, PatentSearcher};
use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{Implementation, ProtocolVersion, ServerCapabilities, ServerInfo, ToolsCapability},
    schemars::{self, JsonSchema},
    service::{NotificationContext, RequestContext},
    tool, tool_handler, tool_router, ErrorData, RoleServer, ServerHandler, ServiceExt,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::io::{stdin, stdout};

/// Request parameters for searching patents
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchPatentsRequest {
    #[schemars(description = "The search query (e.g., 'machine learning')")]
    pub query: Option<String>,

    #[schemars(description = "Filter by assignee/applicant names")]
    pub assignee: Option<Vec<String>>,

    #[schemars(description = "Filter by country code (e.g., 'JP', 'US', 'CN')")]
    pub country: Option<String>,

    #[schemars(description = "Filter by priority date after, format: YYYY-MM-DD")]
    pub after: Option<String>,

    #[schemars(description = "Filter by priority date before, format: YYYY-MM-DD")]
    pub before: Option<String>,

    #[schemars(description = "Maximum number of results to return")]
    pub limit: Option<usize>,

    #[schemars(description = "Language/locale for patent pages (e.g., 'ja', 'en', 'zh')")]
    pub language: Option<String>,
}

/// Request parameters for fetching a patent
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FetchPatentRequest {
    #[schemars(description = "The patent ID (e.g., 'US9152718B2')")]
    pub patent_id: String,

    #[schemars(description = "If true, returns the raw HTML of the patent page")]
    #[serde(default)]
    pub raw: bool,

    #[schemars(description = "Language/locale for patent pages (e.g., 'ja', 'en', 'zh')")]
    pub language: Option<String>,
}

/// MCP handler for Google Patent CLI
#[derive(Clone)]
pub struct PatentHandler {
    tool_router: ToolRouter<PatentHandler>,
    searcher: Arc<dyn PatentSearch>,
}

#[tool_router(router = tool_router)]
impl PatentHandler {
    pub fn new(searcher: Arc<dyn PatentSearch>) -> Self {
        Self { tool_router: Self::tool_router(), searcher }
    }

    /// Search Google Patents for patents matching a query
    #[tool(description = "Search Google Patents for patents matching a query")]
    pub async fn search_patents(
        &self,
        Parameters(request): Parameters<SearchPatentsRequest>,
    ) -> String {
        let options = SearchOptions {
            query: request.query,
            assignee: request.assignee,
            country: request.country,
            patent_number: None,
            after_date: request.after,
            before_date: request.before,
            limit: request.limit,
            language: request.language,
        };

        match self.searcher.search(&options).await {
            Ok(results) => serde_json::to_string_pretty(&results).unwrap_or_default(),
            Err(e) => format!("Search failed: {}", e),
        }
    }

    /// Fetch details of a specific patent by ID
    #[tool(description = "Fetch details of a specific patent by ID")]
    pub async fn fetch_patent(
        &self,
        Parameters(request): Parameters<FetchPatentRequest>,
    ) -> String {
        if request.raw {
            match self.searcher.get_raw_html(&request.patent_id, request.language.as_deref()).await
            {
                Ok(html) => html,
                Err(e) => format!("Failed to fetch raw HTML: {}", e),
            }
        } else {
            let options = SearchOptions {
                query: None,
                assignee: None,
                country: None,
                patent_number: Some(request.patent_id.clone()),
                after_date: None,
                before_date: None,
                limit: None,
                language: request.language,
            };
            match self.searcher.search(&options).await {
                Ok(mut results) => results.patents.pop().map_or_else(
                    || format!("No patent found with ID: {}", request.patent_id),
                    |patent| serde_json::to_string_pretty(&patent).unwrap_or_default(),
                ),
                Err(e) => format!("Fetch failed: {}", e),
            }
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for PatentHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability { list_changed: Some(false) }),
                ..Default::default()
            },
            instructions: Some(
                "Google Patent CLI MCP Server - Search and fetch patents from Google Patents"
                    .to_string(),
            ),
            server_info: Implementation {
                name: "google-patent-cli".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                ..Default::default()
            },
        }
    }

    async fn ping(&self, _ctx: RequestContext<RoleServer>) -> Result<(), ErrorData> {
        Ok(())
    }

    async fn on_initialized(&self, _ctx: NotificationContext<RoleServer>) {
        // Client initialized
    }
}

/// Run the MCP server over stdio
pub async fn run() -> anyhow::Result<()> {
    let config = Config::load()?;
    let searcher = PatentSearcher::new(config.browser_path, true, false)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create PatentSearcher: {}", e))?;
    let handler = PatentHandler::new(Arc::new(searcher));

    let server = handler
        .serve((stdin(), stdout()))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to serve MCP server: {}", e))?;

    server.waiting().await.map_err(|e| anyhow::anyhow!("MCP server error: {}", e))?;

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::core::models::{Patent, SearchResult};

    struct MockSearcher;

    #[async_trait::async_trait]
    impl PatentSearch for MockSearcher {
        async fn search(&self, options: &SearchOptions) -> crate::core::Result<SearchResult> {
            if let Some(pn) = &options.patent_number {
                if pn == "FAIL" {
                    return Err(crate::core::Error::Other("Mock failure".to_string()));
                }
                if pn == "NONE" {
                    return Ok(SearchResult {
                        total_results: "0".to_string(),
                        patents: vec![],
                        top_assignees: None,
                        top_cpcs: None,
                    });
                }
                return Ok(SearchResult {
                    total_results: "1".to_string(),
                    patents: vec![Patent {
                        id: pn.clone(),
                        title: "Mock Patent".to_string(),
                        ..Default::default()
                    }],
                    top_assignees: None,
                    top_cpcs: None,
                });
            }
            Ok(SearchResult {
                total_results: "1".to_string(),
                patents: vec![Patent {
                    id: "SEARCH1".to_string(),
                    title: "Search Result".to_string(),
                    ..Default::default()
                }],
                top_assignees: None,
                top_cpcs: None,
            })
        }

        async fn get_raw_html(
            &self,
            patent_number: &str,
            _language: Option<&str>,
        ) -> crate::core::Result<String> {
            if patent_number == "FAIL" {
                return Err(crate::core::Error::Other("Mock failure".to_string()));
            }
            Ok(format!("<html>{}</html>", patent_number))
        }
    }

    #[tokio::test]
    async fn test_search_patents() {
        let handler = PatentHandler::new(Arc::new(MockSearcher));
        let request = SearchPatentsRequest {
            query: Some("test".to_string()),
            assignee: None,
            country: None,
            after: None,
            before: None,
            limit: None,
            language: None,
        };
        let result = handler.search_patents(Parameters(request)).await;
        assert!(result.contains("SEARCH1"));
        assert!(result.contains("Search Result"));
    }

    #[tokio::test]
    async fn test_fetch_patent() {
        let handler = PatentHandler::new(Arc::new(MockSearcher));

        // Success case
        let request =
            FetchPatentRequest { patent_id: "US123".to_string(), raw: false, language: None };
        let result = handler.fetch_patent(Parameters(request)).await;
        assert!(result.contains("US123"));

        // Raw HTML case
        let request =
            FetchPatentRequest { patent_id: "US123".to_string(), raw: true, language: None };
        let result = handler.fetch_patent(Parameters(request)).await;
        assert_eq!(result, "<html>US123</html>");

        // Not found case
        let request =
            FetchPatentRequest { patent_id: "NONE".to_string(), raw: false, language: None };
        let result = handler.fetch_patent(Parameters(request)).await;
        assert!(result.contains("No patent found"));

        // Error case
        let request =
            FetchPatentRequest { patent_id: "FAIL".to_string(), raw: false, language: None };
        let result = handler.fetch_patent(Parameters(request)).await;
        assert!(result.contains("Fetch failed"));
    }
}
