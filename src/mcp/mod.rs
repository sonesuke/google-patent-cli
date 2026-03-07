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
use std::hash::{DefaultHasher, Hash, Hasher};
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

    // Priority date filters
    #[schemars(description = "Filter by priority date after (YYYY-MM-DD)")]
    pub priority_after: Option<String>,

    #[schemars(description = "Filter by priority date before (YYYY-MM-DD)")]
    pub priority_before: Option<String>,

    // Publication date filters
    #[schemars(description = "Filter by publication date after (YYYY-MM-DD)")]
    pub publication_after: Option<String>,

    #[schemars(description = "Filter by publication date before (YYYY-MM-DD)")]
    pub publication_before: Option<String>,

    // Filing date filters
    #[schemars(description = "Filter by filing date after (YYYY-MM-DD)")]
    pub filing_after: Option<String>,

    #[schemars(description = "Filter by filing date before (YYYY-MM-DD)")]
    pub filing_before: Option<String>,

    #[schemars(description = "Maximum number of results to return")]
    pub limit: Option<usize>,

    #[schemars(description = "Language/locale for patent pages (e.g., 'ja', 'en', 'zh')")]
    pub language: Option<String>,
}

impl Hash for SearchPatentsRequest {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.query.hash(state);
        self.assignee.hash(state);
        self.country.hash(state);
        self.priority_after.hash(state);
        self.priority_before.hash(state);
        self.publication_after.hash(state);
        self.publication_before.hash(state);
        self.filing_after.hash(state);
        self.filing_before.hash(state);
        self.limit.hash(state);
        self.language.hash(state);
    }
}

impl PartialEq for SearchPatentsRequest {
    fn eq(&self, other: &Self) -> bool {
        self.query == other.query
            && self.assignee == other.assignee
            && self.country == other.country
            && self.priority_after == other.priority_after
            && self.priority_before == other.priority_before
            && self.publication_after == other.publication_after
            && self.publication_before == other.publication_before
            && self.filing_after == other.filing_after
            && self.filing_before == other.filing_before
            && self.limit == other.limit
            && self.language == other.language
    }
}

impl Eq for SearchPatentsRequest {}

/// Request parameters for fetching a patent
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Hash, PartialEq, Eq)]
pub struct FetchPatentRequest {
    #[schemars(description = "The patent ID (e.g., 'US9152718B2')")]
    pub patent_id: String,

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

/// Request parameters for patent analyzer skill
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PatentAnalyzerRequest {
    #[schemars(description = "The analysis action to perform")]
    pub action: String,

    #[schemars(description = "Search query (for search, analyze_assignees, prior_art)")]
    pub query: Option<String>,

    #[schemars(description = "Patent ID (for fetch action)")]
    pub patent_id: Option<String>,

    #[schemars(description = "Assignee/Applicant name (for search, check_spelling)")]
    pub assignee: Option<String>,

    #[schemars(description = "Country code (JP, US, CN)")]
    pub country: Option<String>,

    #[schemars(description = "Maximum number of results (default: 10)")]
    pub limit: Option<usize>,

    #[schemars(description = "Return raw HTML (for fetch action)")]
    pub raw: Option<bool>,
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

    /// Generate deterministic dataset name from search request
    fn dataset_name_from_request(request: &SearchPatentsRequest) -> String {
        let mut hasher = DefaultHasher::new();
        request.hash(&mut hasher);
        let hash = hasher.finish();
        format!("search-{:x}", hash)
    }

    /// Generate deterministic dataset name from fetch request
    fn dataset_name_from_fetch(patent_id: &str) -> String {
        let mut hasher = DefaultHasher::new();
        patent_id.hash(&mut hasher);
        let hash = hasher.finish();
        format!("fetch-{:x}", hash)
    }

    /// Evict old datasets if exceeding max cache size
    async fn evict_old_datasets(&self) {
        const MAX_CACHE_SIZE: usize = 100;
        let mut store = self.cypher_store.write().await;
        while store.len() > MAX_CACHE_SIZE {
            // Remove oldest entry (first key)
            if let Some(key) = store.keys().next().cloned() {
                store.remove(&key);
            }
        }
    }

    /// Load JSON data into Cypher store and return graph schema
    async fn load_to_cypher(
        &self,
        name: String,
        json: &Value,
        root_label: Option<&str>,
    ) -> Option<String> {
        let engine = if let Some(label) = root_label {
            CypherEngine::from_json_auto_as_root_with_label(json, label).ok()?
        } else {
            CypherEngine::from_json_auto(json).ok()?
        };
        let graph_schema = engine.get_schema();

        // Store the engine
        let mut store = self.cypher_store.write().await;
        store.insert(name.clone(), engine);

        Some(graph_schema)
    }

    /// Search Google Patents for patents matching a query
    #[tool(description = "Search Google Patents for patents matching a query")]
    pub async fn search_patents(
        &self,
        Parameters(request): Parameters<SearchPatentsRequest>,
    ) -> Result<String, ErrorData> {
        let options = SearchOptions {
            query: request.query.clone(),
            assignee: request.assignee.clone(),
            country: request.country.clone(),
            patent_number: None,
            priority_after: request.priority_after.clone(),
            priority_before: request.priority_before.clone(),
            publication_after: request.publication_after.clone(),
            publication_before: request.publication_before.clone(),
            filing_after: request.filing_after.clone(),
            filing_before: request.filing_before.clone(),
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
        let dataset_name = Self::dataset_name_from_request(&request);
        let graph_schema = self.load_to_cypher(dataset_name.clone(), &json_value, None).await;

        // Evict old datasets if exceeding max cache size
        self.evict_old_datasets().await;

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
        let options = SearchOptions {
            query: None,
            assignee: None,
            country: None,
            patent_number: Some(request.patent_id.clone()),
            priority_after: None,
            priority_before: None,
            publication_after: None,
            publication_before: None,
            filing_after: None,
            filing_before: None,
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
        let dataset_name = Self::dataset_name_from_fetch(&request.patent_id);
        let graph_schema =
            self.load_to_cypher(dataset_name.clone(), &json_value, Some("Patent")).await;

        // Evict old datasets if exceeding max cache size
        self.evict_old_datasets().await;

        let summary = FetchResultSummary {
            output_file: output_path.to_str().unwrap().to_string(),
            schema: serde_json::to_value(schema).unwrap(),
            graph_schema,
            dataset: Some(dataset_name),
        };
        Ok(serde_json::to_string_pretty(&summary).unwrap_or_default())
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
    }

    #[tokio::test]
    async fn test_search_patents() {
        let handler = PatentHandler::new(Arc::new(MockSearcher));
        let request = SearchPatentsRequest {
            query: Some("test".to_string()),
            assignee: None,
            country: None,
            priority_after: None,
            priority_before: None,
            publication_after: None,
            publication_before: None,
            filing_after: None,
            filing_before: None,
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

        // Success case
        let request = FetchPatentRequest { patent_id: "US123".to_string(), language: None };
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

        // Not found case
        let request = FetchPatentRequest { patent_id: "NONE".to_string(), language: None };
        let result = handler.fetch_patent(Parameters(request)).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("No patent found"));

        // Error case
        let request = FetchPatentRequest { patent_id: "FAIL".to_string(), language: None };
        let result = handler.fetch_patent(Parameters(request)).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("Fetch failed"));
    }
}
