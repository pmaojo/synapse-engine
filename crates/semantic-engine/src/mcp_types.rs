use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<serde_json::Value>, // Relaxed from Map to allow Array/Null
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListToolsResult {
    pub tools: Vec<Tool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CallToolResult {
    pub content: Vec<Content>,
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Content {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

// --- Typed Tool Responses ---

#[derive(Serialize, Deserialize, Debug)]
pub struct IngestToolResult {
    pub nodes_added: u32,
    pub edges_added: u32,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SearchResultItem {
    pub node_id: u32,
    pub score: f32,
    pub content: String,
    pub uri: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SearchToolResult {
    pub results: Vec<SearchResultItem>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NeighborItem {
    pub direction: String,
    pub predicate: String,
    pub target: String,
    pub score: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NeighborsToolResult {
    pub neighbors: Vec<NeighborItem>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TripleItem {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TriplesToolResult {
    pub triples: Vec<TripleItem>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReasoningToolResult {
    pub success: bool,
    pub triples_inferred: u32,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SimpleSuccessResult {
    pub success: bool,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StatsToolResult {
    pub active_vectors: usize,
    pub stale_vectors: usize,
    pub total_embeddings: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DegreeResult {
    pub uri: String,
    pub degree: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DisambiguationItem {
    pub uri1: String,
    pub uri2: String,
    pub similarity: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DisambiguationResult {
    pub suggestions: Vec<DisambiguationItem>,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ScenarioItem {
    pub name: String,
    pub description: String,
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ScenarioListResult {
    pub scenarios: Vec<ScenarioItem>,
}
