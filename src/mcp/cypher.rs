use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
