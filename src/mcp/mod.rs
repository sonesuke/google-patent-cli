use crate::core::config::Config;
use crate::core::models::SearchOptions;
use crate::core::patent_search::{PatentSearch, PatentSearcher};
use anyhow::Result;
use async_trait::async_trait;
use mcp_sdk_rs::server::{Server, ServerHandler};
use mcp_sdk_rs::transport::stdio::StdioTransport;
use mcp_sdk_rs::types::{
    ClientCapabilities, Implementation, MessageContent, ServerCapabilities, Tool, ToolResult,
    ToolSchema,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

/// MCP handler for Google Patent CLI
pub struct PatentHandler {
    searcher: Arc<dyn PatentSearch>,
}

impl PatentHandler {
    fn get_tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "search_patents".to_string(),
                description: "Search Google Patents for patents matching a query".to_string(),
                input_schema: Some(ToolSchema {
                    properties: Some(json!({
                        "query": {
                            "type": "string",
                            "description": "The search query (e.g., 'machine learning')"
                        },
                        "assignee": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Filter by assignee/applicant names"
                        },
                        "country": {
                            "type": "string",
                            "description": "Filter by country code (e.g., 'JP', 'US', 'CN')"
                        },
                        "after": {
                            "type": "string",
                            "description": "Filter by priority date after, format: YYYY-MM-DD"
                        },
                        "before": {
                            "type": "string",
                            "description": "Filter by priority date before, format: YYYY-MM-DD"
                        },
                        "limit": {
                            "type": "number",
                            "description": "Maximum number of results to return"
                        },
                        "language": {
                            "type": "string",
                            "description": "Language/locale for patent pages (e.g., 'ja', 'en', 'zh')"
                        }
                    })),
                    required: None,
                }),
                annotations: None,
            },
            Tool {
                name: "fetch_patent".to_string(),
                description: "Fetch details of a specific patent by ID".to_string(),
                input_schema: Some(ToolSchema {
                    properties: Some(json!({
                        "patent_id": {
                            "type": "string",
                            "description": "The patent ID (e.g., 'US9152718B2')"
                        },
                        "raw": {
                            "type": "boolean",
                            "description": "If true, returns the raw HTML of the patent page"
                        },
                        "language": {
                            "type": "string",
                            "description": "Language/locale for patent pages (e.g., 'ja', 'en', 'zh')"
                        }
                    })),
                    required: Some(vec!["patent_id".to_string()]),
                }),
                annotations: None,
            },
        ]
    }

    async fn handle_search_patents(
        &self,
        arguments: &serde_json::Map<String, Value>,
    ) -> ToolResult {
        let query = arguments.get("query").and_then(|v| v.as_str()).map(|s| s.to_string());
        let assignee = arguments.get("assignee").and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        });
        let country = arguments.get("country").and_then(|v| v.as_str()).map(|s| s.to_string());
        let after = arguments.get("after").and_then(|v| v.as_str()).map(|s| s.to_string());
        let before = arguments.get("before").and_then(|v| v.as_str()).map(|s| s.to_string());
        let limit = arguments.get("limit").and_then(|v| v.as_u64()).map(|n| n as usize);
        let language = arguments.get("language").and_then(|v| v.as_str()).map(|s| s.to_string());

        let options = SearchOptions {
            query,
            assignee,
            country,
            patent_number: None,
            after_date: after,
            before_date: before,
            limit,
            language,
        };

        match self.searcher.search(&options).await {
            Ok(results) => ToolResult {
                content: vec![MessageContent::Text {
                    text: serde_json::to_string_pretty(&results).unwrap_or_default(),
                }],
                structured_content: None,
            },
            Err(e) => ToolResult {
                content: vec![MessageContent::Text { text: format!("Search failed: {}", e) }],
                structured_content: None,
            },
        }
    }

    async fn handle_fetch_patent(&self, arguments: &serde_json::Map<String, Value>) -> ToolResult {
        let patent_id = arguments.get("patent_id").and_then(|v| v.as_str()).unwrap_or_default();
        let raw = arguments.get("raw").and_then(|v| v.as_bool()).unwrap_or(false);
        let language = arguments.get("language").and_then(|v| v.as_str());

        if raw {
            match self.searcher.get_raw_html(patent_id, language).await {
                Ok(html) => ToolResult {
                    content: vec![MessageContent::Text { text: html }],
                    structured_content: None,
                },
                Err(e) => ToolResult {
                    content: vec![MessageContent::Text {
                        text: format!("Failed to fetch raw HTML: {}", e),
                    }],
                    structured_content: None,
                },
            }
        } else {
            let options = SearchOptions {
                query: None,
                assignee: None,
                country: None,
                patent_number: Some(patent_id.to_string()),
                after_date: None,
                before_date: None,
                limit: None,
                language: language.map(|s| s.to_string()),
            };
            match self.searcher.search(&options).await {
                Ok(mut results) => results.patents.pop().map_or_else(
                    || ToolResult {
                        content: vec![MessageContent::Text {
                            text: format!("No patent found with ID: {}", patent_id),
                        }],
                        structured_content: None,
                    },
                    |patent| ToolResult {
                        content: vec![MessageContent::Text {
                            text: serde_json::to_string_pretty(&patent).unwrap_or_default(),
                        }],
                        structured_content: None,
                    },
                ),
                Err(e) => ToolResult {
                    content: vec![MessageContent::Text { text: format!("Fetch failed: {}", e) }],
                    structured_content: None,
                },
            }
        }
    }
}

#[async_trait]
impl ServerHandler for PatentHandler {
    async fn initialize(
        &self,
        _implementation: Implementation,
        _capabilities: ClientCapabilities,
    ) -> Result<ServerCapabilities, mcp_sdk_rs::error::Error> {
        Ok(ServerCapabilities {
            tools: Some(json!({
                "listChanged": false
            })),
            ..Default::default()
        })
    }

    async fn shutdown(&self) -> Result<(), mcp_sdk_rs::error::Error> {
        Ok(())
    }

    async fn handle_method(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, mcp_sdk_rs::error::Error> {
        match method {
            "tools/list" => Ok(json!({ "tools": self.get_tools() })),
            "tools/call" => {
                let params = params.ok_or_else(|| {
                    mcp_sdk_rs::error::Error::protocol(
                        mcp_sdk_rs::error::ErrorCode::InvalidParams,
                        "Missing parameters",
                    )
                })?;
                let name = params["name"].as_str().ok_or_else(|| {
                    mcp_sdk_rs::error::Error::protocol(
                        mcp_sdk_rs::error::ErrorCode::InvalidParams,
                        "Missing tool name",
                    )
                })?;
                let arguments = params["arguments"].as_object().ok_or_else(|| {
                    mcp_sdk_rs::error::Error::protocol(
                        mcp_sdk_rs::error::ErrorCode::InvalidParams,
                        "Missing arguments",
                    )
                })?;

                let result = match name {
                    "search_patents" => self.handle_search_patents(arguments).await,
                    "fetch_patent" => self.handle_fetch_patent(arguments).await,
                    _ => {
                        return Err(mcp_sdk_rs::error::Error::protocol(
                            mcp_sdk_rs::error::ErrorCode::MethodNotFound,
                            format!("Unknown tool: {}", name),
                        ))
                    }
                };
                Ok(serde_json::to_value(result).unwrap_or(Value::Null))
            }
            _ => Err(mcp_sdk_rs::error::Error::protocol(
                mcp_sdk_rs::error::ErrorCode::MethodNotFound,
                format!("Unknown method: {}", method),
            )),
        }
    }
}

/// Run the MCP server over stdio
pub async fn run() -> anyhow::Result<()> {
    let config = Config::load()?;
    let searcher = PatentSearcher::new(config.browser_path, true, false)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create PatentSearcher: {}", e))?;
    let handler = Arc::new(PatentHandler { searcher: Arc::new(searcher) });

    let (read_tx, read_rx) = mpsc::channel::<String>(32);
    let (write_tx, mut write_rx) = mpsc::channel::<String>(32);

    // Thread for reading from stdin
    tokio::spawn(async move {
        let stdin = io::stdin();
        let mut reader = BufReader::new(stdin).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let _ = read_tx.send(line).await;
        }
    });

    // Thread for writing to stdout
    tokio::spawn(async move {
        let mut stdout = io::stdout();
        while let Some(line) = write_rx.recv().await {
            let _ = stdout.write_all(line.as_bytes()).await;
            let _ = stdout.write_all(b"\n").await;
            let _ = stdout.flush().await;
        }
    });

    let transport = Arc::new(StdioTransport::new(read_rx, write_tx));

    let server = Server::new(transport, handler);
    server.start().await.map_err(|e| anyhow::anyhow!("MCP server error: {}", e))?;

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::core::models::{Patent, SearchOptions, SearchResult};
    use mcp_sdk_rs::types::MessageContent;

    struct MockSearcher;

    #[async_trait]
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
    async fn test_get_tools() {
        let handler = PatentHandler { searcher: Arc::new(MockSearcher) };
        let tools = handler.get_tools();
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "search_patents");
        assert_eq!(tools[1].name, "fetch_patent");
    }

    #[tokio::test]
    async fn test_handle_search_patents() {
        let handler = PatentHandler { searcher: Arc::new(MockSearcher) };
        let mut args = serde_json::Map::new();
        args.insert("query".to_string(), json!("test"));

        let result = handler.handle_search_patents(&args).await;
        if let MessageContent::Text { text } = &result.content[0] {
            assert!(text.contains("SEARCH1"));
            assert!(text.contains("Search Result"));
        } else {
            panic!("Expected text content");
        }
    }

    #[tokio::test]
    async fn test_handle_fetch_patent() {
        let handler = PatentHandler { searcher: Arc::new(MockSearcher) };

        // Success case
        let mut args = serde_json::Map::new();
        args.insert("patent_id".to_string(), json!("US123"));
        let result = handler.handle_fetch_patent(&args).await;
        if let MessageContent::Text { text } = &result.content[0] {
            assert!(text.contains("US123"));
        } else {
            panic!("Expected text content");
        }

        // Raw HTML case
        let mut args = serde_json::Map::new();
        args.insert("patent_id".to_string(), json!("US123"));
        args.insert("raw".to_string(), json!(true));
        let result = handler.handle_fetch_patent(&args).await;
        if let MessageContent::Text { text } = &result.content[0] {
            assert_eq!(text, "<html>US123</html>");
        }

        // Not found case
        let mut args = serde_json::Map::new();
        args.insert("patent_id".to_string(), json!("NONE"));
        let result = handler.handle_fetch_patent(&args).await;
        if let MessageContent::Text { text } = &result.content[0] {
            assert!(text.contains("No patent found"));
        }

        // Error case
        let mut args = serde_json::Map::new();
        args.insert("patent_id".to_string(), json!("FAIL"));
        let result = handler.handle_fetch_patent(&args).await;
        if let MessageContent::Text { text } = &result.content[0] {
            assert!(text.contains("Fetch failed"));
        }
    }

    #[tokio::test]
    async fn test_handle_method_list() {
        let handler = PatentHandler { searcher: Arc::new(MockSearcher) };
        let res = handler.handle_method("tools/list", None).await.expect("Method handle success");
        assert!(res["tools"].is_array());
        assert_eq!(res["tools"].as_array().expect("Tools is array").len(), 2);
    }

    #[tokio::test]
    async fn test_handle_method_call_search() {
        let handler = PatentHandler { searcher: Arc::new(MockSearcher) };
        let params = json!({
            "name": "search_patents",
            "arguments": { "query": "test" }
        });
        let res =
            handler.handle_method("tools/call", Some(params)).await.expect("Method handle success");
        assert!(res["content"][0]["text"].as_str().expect("Text exists").contains("SEARCH1"));
    }

    #[tokio::test]
    async fn test_handle_method_invalid() {
        let handler = PatentHandler { searcher: Arc::new(MockSearcher) };
        let res = handler.handle_method("unknown", None).await;
        assert!(res.is_err());
    }
}
