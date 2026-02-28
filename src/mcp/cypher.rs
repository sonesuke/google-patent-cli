use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Request parameters for loading JSON from file
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoadJsonFileRequest {
    #[schemars(description = "Unique name/identifier for this dataset")]
    pub name: String,

    #[schemars(description = "Path to JSON file to load")]
    pub file_path: String,

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

/// Request parameters for loading JSON data directly
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoadJsonDataRequest {
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

/// Response for load operations
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoadJsonResponse {
    #[schemars(description = "Name of the loaded dataset")]
    pub name: String,

    #[schemars(description = "Detected graph schema of the JSON data")]
    pub graph_schema: String,

    #[schemars(description = "Number of nodes loaded")]
    pub node_count: usize,
}

/// Request parameters for executing Cypher query
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExecuteCypherRequest {
    #[schemars(description = "Name of the dataset to query")]
    pub dataset: String,

    #[schemars(description = "Cypher query to execute (e.g., 'MATCH (u) RETURN COUNT(u)')")]
    pub query: String,
}

/// Response for execute_cypher
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExecuteCypherResponse {
    #[schemars(description = "Query results as JSON array")]
    pub results: Value,

    #[schemars(description = "Number of rows returned")]
    pub row_count: usize,
}

/// Request parameters for listing loaded datasets
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListDatasetsRequest;

/// Response for list_datasets
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListDatasetsResponse {
    #[schemars(description = "List of loaded dataset names")]
    pub datasets: Vec<String>,

    #[schemars(description = "Total number of datasets")]
    pub count: usize,
}

/// Request parameters for unloading a dataset
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UnloadDatasetRequest {
    #[schemars(description = "Name of the dataset to unload")]
    pub name: String,
}

/// Response for unload_dataset
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UnloadDatasetResponse {
    #[schemars(description = "Confirmation message")]
    pub message: String,

    #[schemars(description = "Name of the unloaded dataset")]
    pub name: String,
}
