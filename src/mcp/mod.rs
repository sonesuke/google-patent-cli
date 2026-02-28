use crate::core::config::Config;
use crate::core::models::SearchOptions;
use crate::core::models::{Patent, SearchResult};
use crate::core::patent_search::{PatentSearch, PatentSearcher};
use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{
        ErrorCode, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo, ToolsCapability,
    },
    schemars::{self, schema_for, JsonSchema},
    service::{NotificationContext, RequestContext},
    tool, tool_handler, tool_router, ErrorData, RoleServer, ServerHandler, ServiceExt,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{stdin, stdout};
use tokio::sync::RwLock;

pub mod cypher;

use cypher_rs::CypherEngine;

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

/// Search result summary for returning to AI
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchResultSummary {
    #[schemars(description = "Path to the output JSON file")]
    pub output_file: String,

    #[schemars(description = "JSON schema of the search results")]
    pub schema: Value,

    #[schemars(description = "Graph schema for Cypher queries")]
    pub graph_schema: Option<String>,

    #[schemars(description = "Dataset name for Cypher queries")]
    pub dataset: Option<String>,

    #[schemars(description = "Number of patents found")]
    pub count: usize,
}

/// Fetch result summary for returning to AI
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FetchResultSummary {
    #[schemars(description = "Path to the output JSON file")]
    pub output_file: String,

    #[schemars(description = "JSON schema of the patent data")]
    pub schema: Value,

    #[schemars(description = "Graph schema for Cypher queries")]
    pub graph_schema: Option<String>,

    #[schemars(description = "Dataset name for Cypher queries")]
    pub dataset: Option<String>,
}

/// MCP handler for Google Patent CLI
#[derive(Clone)]
pub struct PatentHandler {
    tool_router: ToolRouter<PatentHandler>,
    searcher: Arc<dyn PatentSearch>,
    // Cypher store for auto-loading search results
    cypher_store: Arc<RwLock<HashMap<String, CypherEngine>>>,
}

#[tool_router(router = tool_router)]
impl PatentHandler {
    pub fn new(searcher: Arc<dyn PatentSearch>) -> Self {
        Self {
            tool_router: Self::tool_router(),
            searcher,
            cypher_store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Generate a unique dataset name for the search results
    fn generate_dataset_name() -> String {
        format!("patents-{}", uuid::Uuid::new_v4())
    }

    /// Load JSON data into Cypher store and return graph schema
    async fn load_to_cypher(&self, name: String, json: &Value) -> Option<String> {
        let engine = CypherEngine::from_json_auto(json).ok()?;
        let graph_schema = engine.get_schema();

        // Store the engine
        let mut store = self.cypher_store.write().await;
        store.insert(name.clone(), engine);

        Some(graph_schema)
    }

    /// Internal method to load JSON data into a CypherEngine
    async fn load_json_data_internal(
        &self,
        name: String,
        json: Value,
        node_path: Option<String>,
        id_field: Option<String>,
        label_field: Option<String>,
        relation_fields: Option<Vec<String>>,
    ) -> Result<String, ErrorData> {
        // Create engine from JSON
        let engine = if let (Some(node_path), Some(id_field)) = (&node_path, &id_field) {
            // Manual configuration
            let config = cypher_rs::GraphConfig {
                node_path: node_path.clone(),
                id_field: id_field.clone(),
                label_field: label_field.clone(),
                relation_fields: relation_fields.clone().unwrap_or_default(),
            };
            CypherEngine::from_json(&json, config)
        } else {
            // Auto-detection
            CypherEngine::from_json_auto(&json)
        };

        let engine = engine.map_err(|e| {
            ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!("Failed to create Cypher engine: {}", e),
                None,
            )
        })?;

        // Get schema info
        let graph_schema = engine.get_schema();

        // Count nodes
        let node_count = match engine.execute("MATCH (n) RETURN COUNT(n)") {
            Ok(r) => match r.get_single_value() {
                Some(v) => v.as_i64().unwrap_or(0) as usize,
                None => 0,
            },
            Err(_) => 0,
        };

        // Store the engine
        let mut store = self.cypher_store.write().await;
        store.insert(name.clone(), engine);

        let response = cypher::LoadJsonResponse { name, graph_schema, node_count };

        Ok(serde_json::to_string_pretty(&response).unwrap_or_default())
    }

    /// Search Google Patents for patents matching a query
    #[tool(description = "Search Google Patents for patents matching a query")]
    pub async fn search_patents(
        &self,
        Parameters(request): Parameters<SearchPatentsRequest>,
    ) -> Result<String, ErrorData> {
        let options = SearchOptions {
            query: request.query,
            assignee: request.assignee,
            country: request.country,
            patent_number: None,
            after_date: request.after,
            before_date: request.before,
            limit: request.limit,
            language: request.language.clone(),
        };

        let results = self.searcher.search(&options).await.map_err(|e| {
            ErrorData::new(ErrorCode::INTERNAL_ERROR, format!("Search failed: {}", e), None)
        })?;

        let count = results.patents.len();

        // Generate JSON schema for SearchResult
        let schema = schema_for!(SearchResult);

        // Create temp file and write results
        let temp_dir = std::env::temp_dir();
        let file_name = format!("patent-search-{}.json", uuid::Uuid::new_v4());
        let output_path = temp_dir.join(&file_name);
        let json_str = serde_json::to_string_pretty(&results).unwrap_or_default();

        tokio::fs::write(&output_path, &json_str).await.map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to write to file {}: {}", output_path.display(), e),
                None,
            )
        })?;

        // Auto-load into Cypher for querying
        let json_value: Value = serde_json::from_str(&json_str).unwrap_or_default();
        let dataset_name = Self::generate_dataset_name();
        let graph_schema = self.load_to_cypher(dataset_name.clone(), &json_value).await;

        let output_file = output_path.to_str().unwrap().to_string();
        let summary = SearchResultSummary {
            output_file,
            schema: serde_json::to_value(schema).unwrap(),
            graph_schema,
            dataset: Some(dataset_name),
            count,
        };
        Ok(serde_json::to_string_pretty(&summary).unwrap_or_default())
    }

    /// Fetch details of a specific patent by ID
    #[tool(description = "Fetch details of a specific patent by ID")]
    pub async fn fetch_patent(
        &self,
        Parameters(request): Parameters<FetchPatentRequest>,
    ) -> Result<String, ErrorData> {
        if request.raw {
            // Raw HTML mode - write to file and return summary
            let html = self
                .searcher
                .get_raw_html(&request.patent_id, request.language.as_deref())
                .await
                .map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to fetch raw HTML: {}", e),
                        None,
                    )
                })?;

            // Schema for HTML (string type)
            let schema = serde_json::json!({
                "type": "string",
                "description": "Raw HTML of the patent page"
            });

            // Create temp file and write HTML
            let temp_dir = std::env::temp_dir();
            let file_name = format!("patent-{}.html", uuid::Uuid::new_v4());
            let output_path = temp_dir.join(&file_name);

            tokio::fs::write(&output_path, html).await.map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to write to file {}: {}", output_path.display(), e),
                    None,
                )
            })?;

            let summary = FetchResultSummary {
                output_file: output_path.to_str().unwrap().to_string(),
                schema,
                graph_schema: None,
                dataset: None,
            };
            Ok(serde_json::to_string_pretty(&summary).unwrap_or_default())
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
            let mut results = self.searcher.search(&options).await.map_err(|e| {
                ErrorData::new(ErrorCode::INTERNAL_ERROR, format!("Fetch failed: {}", e), None)
            })?;

            let patent = results.patents.pop().ok_or_else(|| {
                ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    format!("No patent found with ID: {}", request.patent_id),
                    None,
                )
            })?;

            // Generate JSON schema for Patent
            let schema = schema_for!(Patent);

            // Create temp file and write results
            let temp_dir = std::env::temp_dir();
            let file_name = format!("patent-{}.json", uuid::Uuid::new_v4());
            let output_path = temp_dir.join(&file_name);
            let json_str = serde_json::to_string_pretty(&patent).unwrap_or_default();

            tokio::fs::write(&output_path, &json_str).await.map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to write to file {}: {}", output_path.display(), e),
                    None,
                )
            })?;

            // Auto-load into Cypher for querying
            let json_value: Value = serde_json::from_str(&json_str).unwrap_or_default();
            let dataset_name = Self::generate_dataset_name();
            let graph_schema = self.load_to_cypher(dataset_name.clone(), &json_value).await;

            let summary = FetchResultSummary {
                output_file: output_path.to_str().unwrap().to_string(),
                schema: serde_json::to_value(schema).unwrap(),
                graph_schema,
                dataset: Some(dataset_name),
            };
            Ok(serde_json::to_string_pretty(&summary).unwrap_or_default())
        }
    }

    /// Execute Cypher query on loaded patent dataset
    #[tool(description = "Execute a Cypher query on a loaded patent dataset")]
    pub async fn execute_cypher(
        &self,
        Parameters(request): Parameters<cypher::ExecuteCypherRequest>,
    ) -> Result<String, ErrorData> {
        // Get the engine
        let store = self.cypher_store.read().await;
        let engine = store.get(&request.dataset).ok_or_else(|| {
            ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!(
                    "Dataset '{}' not found. Run search_patents or fetch_patent first.",
                    request.dataset
                ),
                None,
            )
        })?;

        // Execute query
        let result = engine.execute(&request.query).map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Query execution failed: {}", e),
                None,
            )
        })?;

        // Convert results to JSON array
        let results = result.as_json_array();
        let row_count = results.as_array().map(|a| a.len()).unwrap_or(0);

        let response = cypher::ExecuteCypherResponse { results, row_count };

        Ok(serde_json::to_string_pretty(&response).unwrap_or_default())
    }

    /// List all loaded patent datasets
    #[tool(description = "List all loaded patent datasets")]
    pub async fn list_patent_datasets(
        &self,
        _parameters: Parameters<cypher::ListDatasetsRequest>,
    ) -> Result<String, ErrorData> {
        let store = self.cypher_store.read().await;
        let datasets: Vec<String> = store.keys().cloned().collect();

        let response = cypher::ListDatasetsResponse { datasets, count: store.len() };

        Ok(serde_json::to_string_pretty(&response).unwrap_or_default())
    }

    /// Load JSON file for Cypher querying
    #[tool(description = "Load JSON file and create a queryable dataset with auto-detected schema")]
    pub async fn load_json_file(
        &self,
        Parameters(request): Parameters<cypher::LoadJsonFileRequest>,
    ) -> Result<String, ErrorData> {
        // Read JSON from file
        let file_content = tokio::fs::read_to_string(&request.file_path).await.map_err(|e| {
            ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!("Failed to read file '{}': {}", request.file_path, e),
                None,
            )
        })?;

        let json: Value = serde_json::from_str(&file_content).map_err(|e| {
            ErrorData::new(ErrorCode::INVALID_PARAMS, format!("Failed to parse JSON: {}", e), None)
        })?;

        // Load using internal method
        self.load_json_data_internal(
            request.name,
            json,
            request.node_path,
            request.id_field,
            request.label_field,
            request.relation_fields,
        )
        .await
    }

    /// Load JSON data directly for Cypher querying
    #[tool(
        description = "Load JSON data directly and create a queryable dataset with auto-detected schema"
    )]
    pub async fn load_json_data(
        &self,
        Parameters(request): Parameters<cypher::LoadJsonDataRequest>,
    ) -> Result<String, ErrorData> {
        self.load_json_data_internal(
            request.name,
            request.json,
            request.node_path,
            request.id_field,
            request.label_field,
            request.relation_fields,
        )
        .await
    }

    /// Unload a dataset
    #[tool(description = "Unload a JSON dataset from memory")]
    pub async fn unload_dataset(
        &self,
        Parameters(request): Parameters<cypher::UnloadDatasetRequest>,
    ) -> Result<String, ErrorData> {
        let mut store = self.cypher_store.write().await;
        let existed = store.remove(&request.name).is_some();

        if !existed {
            return Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!("Dataset '{}' not found", request.name),
                None,
            ));
        }

        let response = cypher::UnloadDatasetResponse {
            message: format!("Dataset '{}' unloaded successfully", request.name),
            name: request.name,
        };

        Ok(serde_json::to_string_pretty(&response).unwrap_or_default())
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
    let (browser_path, chrome_args) = config.resolve();
    let searcher = PatentSearcher::new(browser_path, true, false, false, chrome_args)
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
        assert!(result.is_ok());
        let result_str = result.unwrap();

        // Should contain summary with output_file and schema
        assert!(result_str.contains("\"output_file\""));
        assert!(result_str.contains("\"schema\""));

        // Extract file path and schema from JSON
        let summary: SearchResultSummary = serde_json::from_str(&result_str).unwrap();
        assert!(summary.output_file.starts_with('/')); // Absolute path
        assert!(summary.schema.is_object()); // Schema is a JSON object

        let file_content = tokio::fs::read_to_string(&summary.output_file).await.unwrap();
        assert!(file_content.contains("SEARCH1"));
        assert!(file_content.contains("Search Result"));

        // Clean up
        let _ = tokio::fs::remove_file(&summary.output_file).await;
    }

    #[tokio::test]
    async fn test_fetch_patent() {
        let handler = PatentHandler::new(Arc::new(MockSearcher));

        // Success case (JSON mode)
        let request =
            FetchPatentRequest { patent_id: "US123".to_string(), raw: false, language: None };
        let result = handler.fetch_patent(Parameters(request)).await;
        assert!(result.is_ok());
        let result_str = result.unwrap();

        // Should contain summary with file path and schema
        assert!(result_str.contains("\"output_file\""));
        assert!(result_str.contains("\"schema\""));

        let summary: FetchResultSummary = serde_json::from_str(&result_str).unwrap();
        assert!(summary.output_file.starts_with('/')); // Absolute path
        assert!(summary.schema.is_object()); // Schema is a JSON object

        // Clean up
        let _ = tokio::fs::remove_file(&summary.output_file).await;

        // Raw HTML case - also writes to file and returns summary
        let request =
            FetchPatentRequest { patent_id: "US123".to_string(), raw: true, language: None };
        let result = handler.fetch_patent(Parameters(request)).await;
        assert!(result.is_ok());
        let result_str = result.unwrap();

        // Should contain summary with file path and schema
        assert!(result_str.contains("\"output_file\""));
        assert!(result_str.contains("\"schema\""));

        let summary: FetchResultSummary = serde_json::from_str(&result_str).unwrap();
        assert!(summary.output_file.starts_with('/')); // Absolute path
        assert!(summary.schema.is_object()); // Schema is a JSON object

        // Verify HTML file
        let file_content = tokio::fs::read_to_string(&summary.output_file).await.unwrap();
        assert!(file_content.contains("<html>US123</html>"));

        // Clean up
        let _ = tokio::fs::remove_file(&summary.output_file).await;

        // Not found case
        let request =
            FetchPatentRequest { patent_id: "NONE".to_string(), raw: false, language: None };
        let result = handler.fetch_patent(Parameters(request)).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("No patent found"));

        // Error case
        let request =
            FetchPatentRequest { patent_id: "FAIL".to_string(), raw: false, language: None };
        let result = handler.fetch_patent(Parameters(request)).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("Fetch failed"));
    }
}
