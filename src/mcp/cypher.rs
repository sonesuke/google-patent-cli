use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters, ServerHandler},
    model::ErrorCode,
    schemars::{self, JsonSchema},
    service::{NotificationContext, RequestContext},
    tool, tool_handler, tool_router, ErrorData, RoleServer,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use cypher_rs::CypherEngine;

/// Store for loaded JSON datasets
type JsonStore = Arc<RwLock<HashMap<String, CypherEngine>>>;

/// Request parameters for loading JSON
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoadJsonRequest {
    #[schemars(description = "Unique name/identifier for this dataset")]
    pub name: String,

    #[schemars(description = "JSON data to load (object or array)")]
    pub json: Value,

    #[schemars(
        description = "JSON path to node array (e.g., 'data.users'). If not provided, auto-detection will be used"
    )]
    pub node_path: Option<String>,

    #[schemars(
        description = "Field containing unique ID (e.g., 'id'). If not provided, auto-detection will be used"
    )]
    pub id_field: Option<String>,

    #[schemars(
        description = "Field containing node label for typing (e.g., 'type', 'role'). If not provided, auto-detection will be used"
    )]
    pub label_field: Option<String>,

    #[schemars(
        description = "Fields containing relationship arrays (e.g., ['friends', 'connections']). If not provided, auto-detection will be used"
    )]
    pub relation_fields: Option<Vec<String>>,
}

/// Response for load_json
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoadJsonResponse {
    #[schemars(description = "Name of the loaded dataset")]
    pub name: String,

    #[schemars(description = "Detected schema of the JSON data")]
    pub schema: String,

    #[schemars(description = "Number of nodes loaded")]
    pub node_count: usize,
}

/// Request parameters for querying JSON
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QueryJsonRequest {
    #[schemars(description = "Name of the dataset to query")]
    pub dataset: String,

    #[schemars(description = "Cypher query to execute (e.g., 'MATCH (u) RETURN COUNT(u)')")]
    pub query: String,
}

/// Response for query_json
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QueryJsonResponse {
    #[schemars(description = "Query results as JSON array")]
    pub results: Value,

    #[schemars(description = "Number of rows returned")]
    pub row_count: usize,
}

/// Request parameters for listing loaded datasets
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListLoadedRequest;

/// Response for list_loaded
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListLoadedResponse {
    #[schemars(description = "List of loaded dataset names")]
    pub datasets: Vec<String>,

    #[schemars(description = "Total number of datasets")]
    pub count: usize,
}

/// Request parameters for unloading a dataset
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UnloadJsonRequest {
    #[schemars(description = "Name of the dataset to unload")]
    pub name: String,
}

/// Response for unload_json
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UnloadJsonResponse {
    #[schemars(description = "Confirmation message")]
    pub message: String,

    #[schemars(description = "Name of the unloaded dataset")]
    pub name: String,
}

/// MCP handler for Cypher JSON Query
#[derive(Clone)]
pub struct CypherHandler {
    tool_router: ToolRouter<CypherHandler>,
    store: JsonStore,
}

impl Default for CypherHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router(router = tool_router)]
impl CypherHandler {
    pub fn new() -> Self {
        Self { tool_router: Self::tool_router(), store: Arc::new(RwLock::new(HashMap::new())) }
    }

    /// Load JSON data for Cypher querying
    #[tool(description = "Load JSON data and create a queryable dataset with auto-detected schema")]
    pub async fn load_json(
        &self,
        Parameters(request): Parameters<LoadJsonRequest>,
    ) -> Result<String, ErrorData> {
        // Create engine from JSON
        let engine =
            if let (Some(node_path), Some(id_field)) = (&request.node_path, &request.id_field) {
                // Manual configuration
                let config = cypher_rs::GraphConfig {
                    node_path: node_path.clone(),
                    id_field: id_field.clone(),
                    label_field: request.label_field.clone(),
                    relation_fields: request.relation_fields.clone().unwrap_or_default(),
                };
                CypherEngine::from_json(&request.json, config)
            } else {
                // Auto-detection
                CypherEngine::from_json_auto(&request.json)
            };

        let engine = engine.map_err(|e| {
            ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!("Failed to create Cypher engine: {}", e),
                None,
            )
        })?;

        // Get schema info
        let schema = engine.get_schema();

        // Count nodes
        let node_count = match engine.execute("MATCH (n) RETURN COUNT(n)") {
            Ok(r) => match r.get_single_value() {
                Some(v) => v.as_i64().unwrap_or(0) as usize,
                None => 0,
            },
            Err(_) => 0,
        };

        // Store the engine
        let mut store = self.store.write().await;
        store.insert(request.name.clone(), engine);

        let response = LoadJsonResponse { name: request.name, schema, node_count };

        Ok(serde_json::to_string_pretty(&response).unwrap_or_default())
    }

    /// Query loaded JSON data with Cypher
    #[tool(description = "Execute a Cypher query on a loaded JSON dataset")]
    pub async fn query_json(
        &self,
        Parameters(request): Parameters<QueryJsonRequest>,
    ) -> Result<String, ErrorData> {
        // Get the engine
        let store = self.store.read().await;
        let engine = store.get(&request.dataset).ok_or_else(|| {
            ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!("Dataset '{}' not found. Use load_json first.", request.dataset),
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

        let response = QueryJsonResponse { results, row_count };

        Ok(serde_json::to_string_pretty(&response).unwrap_or_default())
    }

    /// List all loaded JSON datasets
    #[tool(description = "List all loaded JSON datasets")]
    pub async fn list_loaded(
        &self,
        _parameters: Parameters<ListLoadedRequest>,
    ) -> Result<String, ErrorData> {
        let store = self.store.read().await;
        let datasets: Vec<String> = store.keys().cloned().collect();

        let response = ListLoadedResponse { datasets, count: store.len() };

        Ok(serde_json::to_string_pretty(&response).unwrap_or_default())
    }

    /// Unload a JSON dataset
    #[tool(description = "Unload a JSON dataset from memory")]
    pub async fn unload_json(
        &self,
        Parameters(request): Parameters<UnloadJsonRequest>,
    ) -> Result<String, ErrorData> {
        let mut store = self.store.write().await;
        let existed = store.remove(&request.name).is_some();

        if !existed {
            return Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                format!("Dataset '{}' not found", request.name),
                None,
            ));
        }

        let response = UnloadJsonResponse {
            message: format!("Dataset '{}' unloaded successfully", request.name),
            name: request.name,
        };

        Ok(serde_json::to_string_pretty(&response).unwrap_or_default())
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for CypherHandler {
    fn get_info(&self) -> rmcp::model::ServerInfo {
        rmcp::model::ServerInfo {
            protocol_version: rmcp::model::ProtocolVersion::V_2024_11_05,
            capabilities: rmcp::model::ServerCapabilities {
                tools: Some(rmcp::model::ToolsCapability { list_changed: Some(false) }),
                ..Default::default()
            },
            instructions: Some(
                "Cypher JSON Query MCP Server - Query JSON data using Cypher syntax".to_string(),
            ),
            server_info: rmcp::model::Implementation {
                name: "cypher-json-query".into(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load_and_query_json() {
        let handler = CypherHandler::new();

        // Load JSON data
        let json_data = serde_json::json!({
            "users": [
                {"id": "1", "name": "Alice", "role": "admin", "age": 30},
                {"id": "2", "name": "Bob", "role": "user", "age": 25}
            ]
        });

        let load_request = LoadJsonRequest {
            name: "test".to_string(),
            json: json_data,
            node_path: None,
            id_field: None,
            label_field: None,
            relation_fields: None,
        };

        let result = handler.load_json(Parameters(load_request)).await;
        assert!(result.is_ok());

        // Query the data
        let query_request = QueryJsonRequest {
            dataset: "test".to_string(),
            query: "MATCH (u) RETURN COUNT(u)".to_string(),
        };

        let result = handler.query_json(Parameters(query_request)).await;
        assert!(result.is_ok());

        // List datasets
        let result = handler.list_loaded(Parameters(ListLoadedRequest)).await;
        assert!(result.is_ok());

        // Unload
        let unload_request = UnloadJsonRequest { name: "test".to_string() };
        let result = handler.unload_json(Parameters(unload_request)).await;
        assert!(result.is_ok());
    }
}
