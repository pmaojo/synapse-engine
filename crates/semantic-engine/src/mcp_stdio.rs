use crate::mcp_types::{
    CallToolResult, Content, DegreeResult, DisambiguationItem, DisambiguationResult,
    IngestToolResult, ListToolsResult, McpError, McpRequest, McpResponse, NeighborItem,
    NeighborsToolResult, ReasoningToolResult, SearchResultItem, SearchToolResult,
    SimpleSuccessResult, StatsToolResult, Tool, TripleItem, TriplesToolResult,
};
use crate::server::proto::semantic_engine_server::SemanticEngine;
use crate::server::proto::{
    HybridSearchRequest, IngestFileRequest, IngestRequest, Provenance, ReasoningRequest,
    ReasoningStrategy, SearchMode, SparqlRequest, Triple,
};
use crate::server::MySemanticEngine;
use jsonschema::JSONSchema;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tonic::Request;

pub struct McpStdioServer {
    engine: Arc<MySemanticEngine>,
}

impl McpStdioServer {
    pub fn new(engine: Arc<MySemanticEngine>) -> Self {
        Self { engine }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut reader = BufReader::new(tokio::io::stdin());
        let mut writer = tokio::io::stdout();

        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).await? == 0 {
                break;
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if let Ok(request) = serde_json::from_str::<McpRequest>(trimmed) {
                let response = self.handle_request(request).await;
                let response_json = serde_json::to_string(&response)? + "\n";
                writer.write_all(response_json.as_bytes()).await?;
                writer.flush().await?;
            }
        }

        Ok(())
    }

    fn create_request<T>(msg: T) -> Request<T> {
        let mut req = Request::new(msg);

        // Try to get token from env
        let token_opt = std::env::var("SYNAPSE_ADMIN_TOKEN")
            .or_else(|_| std::env::var("SYNAPSE_MCP_TOKEN"))
            .ok();

        if let Some(token) = token_opt {
            if let Ok(val) = format!("Bearer {}", token).parse() {
                req.metadata_mut().insert("authorization", val);
            }
        }
        req
    }

    fn get_tools() -> Vec<Tool> {
        vec![
            Tool {
                name: "ingest_triples".to_string(),
                description: Some(
                    "Ingest one or more RDF triples into the knowledge graph".to_string(),
                ),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "triples": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "subject": { "type": "string" },
                                    "predicate": { "type": "string" },
                                    "object": { "type": "string" }
                                },
                                "required": ["subject", "predicate", "object"]
                            }
                        },
                        "namespace": { "type": "string", "default": "default" }
                    },
                    "required": ["triples"]
                }),
            },
            Tool {
                name: "ingest_file".to_string(),
                description: Some(
                    "Ingest a CSV or Markdown file into the knowledge graph".to_string(),
                ),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Path to the file" },
                        "namespace": { "type": "string", "default": "default" }
                    },
                    "required": ["path"]
                }),
            },
            Tool {
                name: "sparql_query".to_string(),
                description: Some("Execute a SPARQL query against the knowledge graph".to_string()),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "SPARQL query string" },
                        "namespace": { "type": "string", "default": "default" }
                    },
                    "required": ["query"]
                }),
            },
            Tool {
                name: "hybrid_search".to_string(),
                description: Some("Perform a hybrid vector + graph search".to_string()),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Natural language query" },
                        "namespace": { "type": "string", "default": "default" },
                        "vector_k": { "type": "integer", "default": 10 },
                        "graph_depth": { "type": "integer", "default": 1 },
                        "limit": { "type": "integer", "default": 20 }
                    },
                    "required": ["query"]
                }),
            },
            Tool {
                name: "apply_reasoning".to_string(),
                description: Some(
                    "Apply RDFS or OWL-RL reasoning to infer new triples".to_string(),
                ),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "namespace": { "type": "string", "default": "default" },
                        "strategy": { "type": "string", "enum": ["rdfs", "owlrl"], "default": "rdfs" },
                        "materialize": { "type": "boolean", "default": false }
                    }
                }),
            },
            Tool {
                name: "get_neighbors".to_string(),
                description: Some(
                    "Get neighboring nodes connected to a given URI in the graph".to_string(),
                ),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "uri": { "type": "string", "description": "URI of the entity to find neighbors for" },
                        "namespace": { "type": "string", "default": "default" },
                        "direction": { "type": "string", "enum": ["outgoing", "incoming", "both"], "default": "outgoing" }
                    },
                    "required": ["uri"]
                }),
            },
            Tool {
                name: "list_triples".to_string(),
                description: Some(
                    "List all triples in a namespace (useful for debugging/exploration)"
                        .to_string(),
                ),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "namespace": { "type": "string", "default": "default" },
                        "limit": { "type": "integer", "default": 100 }
                    }
                }),
            },
            Tool {
                name: "delete_namespace".to_string(),
                description: Some("Delete all data in a namespace".to_string()),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "namespace": { "type": "string", "description": "Namespace to delete" }
                    },
                    "required": ["namespace"]
                }),
            },
            Tool {
                name: "ingest_url".to_string(),
                description: Some(
                    "Scrape a web page and add its content to the vector store for RAG retrieval"
                        .to_string(),
                ),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": { "type": "string", "description": "URL to scrape and ingest" },
                        "namespace": { "type": "string", "default": "default" }
                    },
                    "required": ["url"]
                }),
            },
            Tool {
                name: "ingest_text".to_string(),
                description: Some(
                    "Add arbitrary text content to the vector store for RAG retrieval".to_string(),
                ),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "uri": { "type": "string", "description": "Custom URI identifier for this text" },
                        "content": { "type": "string", "description": "Text content to embed and store" },
                        "namespace": { "type": "string", "default": "default" }
                    },
                    "required": ["uri", "content"]
                }),
            },
            Tool {
                name: "compact_vectors".to_string(),
                description: Some("Compact the vector index by removing stale entries".to_string()),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "namespace": { "type": "string", "default": "default" }
                    }
                }),
            },
            Tool {
                name: "vector_stats".to_string(),
                description: Some("Get vector store statistics (active, stale, total)".to_string()),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "namespace": { "type": "string", "default": "default" }
                    }
                }),
            },
            Tool {
                name: "disambiguate".to_string(),
                description: Some("Find similar entities that might be duplicates".to_string()),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "namespace": { "type": "string", "default": "default" },
                        "threshold": { "type": "number", "default": 0.8, "description": "Similarity threshold 0.0-1.0" }
                    }
                }),
            },
            Tool {
                name: "get_node_degree".to_string(),
                description: Some("Get the degree (number of connections) of a node".to_string()),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "uri": { "type": "string" },
                        "namespace": { "type": "string", "default": "default" }
                    },
                    "required": ["uri"]
                }),
            },
            Tool {
                name: "install_ontology".to_string(),
                description: Some("Download and install an ontology from a URL".to_string()),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": { "type": "string", "description": "URL of the ontology file (.owl, .ttl)" },
                        "name": { "type": "string", "description": "Name for the local file (e.g. 'legal.owl')" },
                        "namespace": { "type": "string", "default": "default" }
                    },
                    "required": ["url", "name"]
                }),
            },
        ]
    }

    pub async fn handle_request(&self, request: McpRequest) -> McpResponse {
        match request.method.as_str() {
            "initialize" => {
                // MCP protocol initialization
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(serde_json::json!({
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {}
                        },
                        "serverInfo": {
                        "name": "synapse",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                    })),
                    error: None,
                }
            }
            "notifications/initialized" | "initialized" => {
                // Client confirms initialization - just acknowledge
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(serde_json::json!({})),
                    error: None,
                }
            }
            "tools/list" => {
                let result = ListToolsResult {
                    tools: Self::get_tools(),
                };
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(serde_json::to_value(result).unwrap()),
                    error: None,
                }
            }
            "tools/call" => self.handle_tool_call(request).await,
            // Legacy methods for backwards compatibility
            "ingest" => self.handle_legacy_ingest(request).await,
            "ingest_file" => self.handle_legacy_ingest_file(request).await,
            _ => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(McpError {
                    code: -32601,
                    message: format!("Method not found: {}", request.method),
                    data: None,
                }),
            },
        }
    }

    fn validate_arguments(tool_name: &str, arguments: &serde_json::Value) -> Result<(), String> {
        let tools = Self::get_tools();
        if let Some(tool) = tools.iter().find(|t| t.name == tool_name) {
            if let Ok(schema) = JSONSchema::compile(&tool.input_schema) {
                if let Err(errors) = schema.validate(arguments) {
                    let error_msg = errors.map(|e| e.to_string()).collect::<Vec<_>>().join(", ");
                    return Err(format!("Validation error: {}", error_msg));
                }
            } else {
                return Err("Invalid tool schema definition".to_string());
            }
        }
        Ok(())
    }

    async fn handle_tool_call(&self, request: McpRequest) -> McpResponse {
        let params = match request.params {
            Some(p) => p,
            None => return self.error_response(request.id, -32602, "Missing params"),
        };

        let tool_name = match params.get("name").and_then(|v| v.as_str()) {
            Some(n) => n,
            None => return self.error_response(request.id, -32602, "Missing tool name"),
        };

        let arguments = params
            .get("arguments")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        let args_value = serde_json::Value::Object(arguments.clone());
        if let Err(e) = Self::validate_arguments(tool_name, &args_value) {
            return self.error_response(request.id, -32602, &e);
        }

        match tool_name {
            "ingest_triples" => self.call_ingest_triples(request.id, &arguments).await,
            "ingest_file" => self.call_ingest_file(request.id, &arguments).await,
            "sparql_query" => self.call_sparql_query(request.id, &arguments).await,
            "hybrid_search" => self.call_hybrid_search(request.id, &arguments).await,
            "apply_reasoning" => self.call_apply_reasoning(request.id, &arguments).await,
            "get_neighbors" => self.call_get_neighbors(request.id, &arguments).await,
            "list_triples" => self.call_list_triples(request.id, &arguments).await,
            "delete_namespace" => self.call_delete_namespace(request.id, &arguments).await,
            "ingest_url" => self.call_ingest_url(request.id, &arguments).await,
            "ingest_text" => self.call_ingest_text(request.id, &arguments).await,
            "compact_vectors" => self.call_compact_vectors(request.id, &arguments).await,
            "vector_stats" => self.call_vector_stats(request.id, &arguments).await,
            "disambiguate" => self.call_disambiguate(request.id, &arguments).await,
            "get_node_degree" => self.call_get_node_degree(request.id, &arguments).await,
            "install_ontology" => self.call_install_ontology(request.id, &arguments).await,
            _ => self.error_response(request.id, -32602, &format!("Unknown tool: {}", tool_name)),
        }
    }

    async fn call_install_ontology(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Map<String, serde_json::Value>,
    ) -> McpResponse {
        let url = match args.get("url").and_then(|v| v.as_str()) {
            Some(u) => u,
            None => return self.error_response(id, -32602, "Missing 'url'"),
        };
        let name = match args.get("name").and_then(|v| v.as_str()) {
            Some(n) => n,
            None => return self.error_response(id, -32602, "Missing 'name'"),
        };
        let namespace = args
            .get("namespace")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        // Download
        let response = match reqwest::get(url).await {
            Ok(r) => r,
            Err(e) => {
                return self.tool_result(id, &format!("Failed to download ontology: {}", e), true)
            }
        };

        if !response.status().is_success() {
            return self.tool_result(id, &format!("HTTP error: {}", response.status()), true);
        }

        let content = match response.text().await {
            Ok(t) => t,
            Err(e) => {
                return self.tool_result(id, &format!("Failed to read response: {}", e), true)
            }
        };

        // Ensure ontology directory exists
        let ontology_dir = std::path::Path::new("ontology");
        if !ontology_dir.exists() {
            if let Err(e) = std::fs::create_dir(ontology_dir) {
                return self.tool_result(
                    id,
                    &format!("Failed to create ontology dir: {}", e),
                    true,
                );
            }
        }

        // Security check: Sanitize filename to prevent directory traversal
        let file_name = std::path::Path::new(name)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| "Invalid filename".to_string());

        let clean_name = match file_name {
            Ok(n) => n,
            Err(e) => return self.tool_result(id, &format!("Security error: {}", e), true),
        };

        if clean_name != name {
            return self.tool_result(
                id,
                "Security error: Filename contains path components",
                true,
            );
        }

        let path = ontology_dir.join(clean_name);
        if let Err(e) = std::fs::write(&path, content) {
            return self.tool_result(id, &format!("Failed to save ontology file: {}", e), true);
        }

        // Load into store
        let store = match self.engine.get_store(namespace) {
            Ok(s) => s,
            Err(e) => return self.tool_result(id, &e.to_string(), true),
        };

        match crate::ingest::ontology::OntologyLoader::load_file(&store, &path).await {
            Ok(count) => {
                let result = SimpleSuccessResult {
                    success: true,
                    message: format!("Installed ontology '{}' and loaded {} triples", name, count),
                };
                self.serialize_result(id, result)
            }
            Err(e) => self.tool_result(id, &format!("Failed to load ontology: {}", e), true),
        }
    }

    async fn call_ingest_triples(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Map<String, serde_json::Value>,
    ) -> McpResponse {
        let namespace = args
            .get("namespace")
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        let triples_array = match args.get("triples").and_then(|v| v.as_array()) {
            Some(t) => t,
            None => return self.error_response(id, -32602, "Missing 'triples' array"),
        };

        let mut triples = Vec::new();
        for t in triples_array {
            if let (Some(s), Some(p), Some(o)) = (
                t.get("subject").and_then(|v| v.as_str()),
                t.get("predicate").and_then(|v| v.as_str()),
                t.get("object").and_then(|v| v.as_str()),
            ) {
                triples.push(Triple {
                    subject: s.to_string(),
                    predicate: p.to_string(),
                    object: o.to_string(),
                    provenance: Some(Provenance {
                        source: "mcp".to_string(),
                        timestamp: "".to_string(),
                        method: "tools/call".to_string(),
                    }),
                    embedding: vec![],
                });
            }
        }

        let req = Self::create_request(IngestRequest {
            triples,
            namespace: namespace.to_string(),
        });

        match self.engine.ingest_triples(req).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let result = IngestToolResult {
                    nodes_added: inner.nodes_added,
                    edges_added: inner.edges_added,
                    message: format!("Ingested {} triples", inner.edges_added),
                };
                self.serialize_result(id, result)
            }
            Err(e) => self.tool_result(id, &e.to_string(), true),
        }
    }

    async fn call_ingest_file(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Map<String, serde_json::Value>,
    ) -> McpResponse {
        let path = match args.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return self.error_response(id, -32602, "Missing 'path'"),
        };
        let namespace = args
            .get("namespace")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        let req = Self::create_request(IngestFileRequest {
            file_path: path.to_string(),
            namespace: namespace.to_string(),
        });

        match self.engine.ingest_file(req).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let result = IngestToolResult {
                    nodes_added: inner.nodes_added,
                    edges_added: inner.edges_added,
                    message: format!("Ingested {} triples from {}", inner.edges_added, path),
                };
                self.serialize_result(id, result)
            }
            Err(e) => self.tool_result(id, &e.to_string(), true),
        }
    }

    async fn call_sparql_query(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Map<String, serde_json::Value>,
    ) -> McpResponse {
        let query = match args.get("query").and_then(|v| v.as_str()) {
            Some(q) => q,
            None => return self.error_response(id, -32602, "Missing 'query'"),
        };
        let namespace = args
            .get("namespace")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        let req = Self::create_request(SparqlRequest {
            query: query.to_string(),
            namespace: namespace.to_string(),
        });

        match self.engine.query_sparql(req).await {
            Ok(resp) => self.tool_result(id, &resp.into_inner().results_json, false),
            Err(e) => self.tool_result(id, &e.to_string(), true),
        }
    }

    async fn call_hybrid_search(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Map<String, serde_json::Value>,
    ) -> McpResponse {
        let query = match args.get("query").and_then(|v| v.as_str()) {
            Some(q) => q,
            None => return self.error_response(id, -32602, "Missing 'query'"),
        };
        let namespace = args
            .get("namespace")
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        let vector_k = args.get("vector_k").and_then(|v| v.as_u64()).unwrap_or(10) as u32;
        let graph_depth = args
            .get("graph_depth")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as u32;
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as u32;

        let req = Self::create_request(HybridSearchRequest {
            query: query.to_string(),
            namespace: namespace.to_string(),
            vector_k,
            graph_depth,
            mode: SearchMode::Hybrid as i32,
            limit,
        });

        match self.engine.hybrid_search(req).await {
            Ok(resp) => {
                let results = resp.into_inner().results;
                let items: Vec<SearchResultItem> = results
                    .into_iter()
                    .map(|r| SearchResultItem {
                        node_id: r.node_id,
                        score: r.score,
                        content: r.content,
                        uri: r.uri,
                    })
                    .collect();

                let result = SearchToolResult { results: items };
                self.serialize_result(id, result)
            }
            Err(e) => self.tool_result(id, &e.to_string(), true),
        }
    }

    async fn call_apply_reasoning(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Map<String, serde_json::Value>,
    ) -> McpResponse {
        let namespace = args
            .get("namespace")
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        let strategy_str = args
            .get("strategy")
            .and_then(|v| v.as_str())
            .unwrap_or("rdfs");
        let materialize = args
            .get("materialize")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let strategy = match strategy_str.to_lowercase().as_str() {
            "owlrl" | "owl-rl" => ReasoningStrategy::Owlrl as i32,
            _ => ReasoningStrategy::Rdfs as i32,
        };

        let req = Self::create_request(ReasoningRequest {
            namespace: namespace.to_string(),
            strategy,
            materialize,
        });

        match self.engine.apply_reasoning(req).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let result = ReasoningToolResult {
                    success: inner.success,
                    triples_inferred: inner.triples_inferred,
                    message: inner.message,
                };
                self.serialize_result(id, result)
            }
            Err(e) => self.tool_result(id, &e.to_string(), true),
        }
    }

    async fn call_get_neighbors(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Map<String, serde_json::Value>,
    ) -> McpResponse {
        let uri = match args.get("uri").and_then(|v| v.as_str()) {
            Some(u) => u,
            None => return self.error_response(id, -32602, "Missing 'uri'"),
        };
        let namespace = args
            .get("namespace")
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        let direction = args
            .get("direction")
            .and_then(|v| v.as_str())
            .unwrap_or("outgoing");

        let store = match self.engine.get_store(namespace) {
            Ok(s) => s,
            Err(e) => return self.tool_result(id, &e.to_string(), true),
        };

        let mut neighbors = Vec::new();

        // Query outgoing edges (URI as subject)
        if direction == "outgoing" || direction == "both" {
            if let Ok(subj) = oxigraph::model::NamedNodeRef::new(uri) {
                for q in store
                    .store
                    .quads_for_pattern(Some(subj.into()), None, None, None)
                    .flatten()
                {
                    neighbors.push(NeighborItem {
                        direction: "outgoing".to_string(),
                        predicate: q.predicate.to_string(),
                        target: q.object.to_string(),
                        score: 1.0,
                    });
                }
            }
        }

        // Query incoming edges (URI as object)
        if direction == "incoming" || direction == "both" {
            if let Ok(obj) = oxigraph::model::NamedNodeRef::new(uri) {
                for q in store
                    .store
                    .quads_for_pattern(None, None, Some(obj.into()), None)
                    .flatten()
                {
                    neighbors.push(NeighborItem {
                        direction: "incoming".to_string(),
                        predicate: q.predicate.to_string(),
                        target: q.subject.to_string(),
                        score: 1.0,
                    });
                }
            }
        }

        let result = NeighborsToolResult { neighbors };
        self.serialize_result(id, result)
    }

    async fn call_list_triples(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Map<String, serde_json::Value>,
    ) -> McpResponse {
        let namespace = args
            .get("namespace")
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as usize;

        let store = match self.engine.get_store(namespace) {
            Ok(s) => s,
            Err(e) => return self.tool_result(id, &e.to_string(), true),
        };

        let mut triples = Vec::new();
        for q in store.store.iter().take(limit).flatten() {
            triples.push(TripleItem {
                subject: q.subject.to_string(),
                predicate: q.predicate.to_string(),
                object: q.object.to_string(),
            });
        }

        let result = TriplesToolResult { triples };
        self.serialize_result(id, result)
    }

    async fn call_delete_namespace(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Map<String, serde_json::Value>,
    ) -> McpResponse {
        let namespace = match args.get("namespace").and_then(|v| v.as_str()) {
            Some(n) => n,
            None => return self.error_response(id, -32602, "Missing 'namespace'"),
        };

        let req = Self::create_request(crate::server::proto::EmptyRequest {
            namespace: namespace.to_string(),
        });

        match self.engine.delete_namespace_data(req).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                let result = SimpleSuccessResult {
                    success: inner.success,
                    message: inner.message,
                };
                self.serialize_result(id, result)
            }
            Err(e) => self.tool_result(id, &e.to_string(), true),
        }
    }

    async fn call_ingest_url(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Map<String, serde_json::Value>,
    ) -> McpResponse {
        let url = match args.get("url").and_then(|v| v.as_str()) {
            Some(u) => u,
            None => return self.error_response(id, -32602, "Missing 'url'"),
        };
        let namespace = args
            .get("namespace")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        // Fetch URL content
        let client = reqwest::Client::new();
        let response = match client.get(url).send().await {
            Ok(r) => r,
            Err(e) => return self.tool_result(id, &format!("Failed to fetch URL: {}", e), true),
        };

        if !response.status().is_success() {
            return self.tool_result(id, &format!("HTTP error: {}", response.status()), true);
        }

        let html = match response.text().await {
            Ok(t) => t,
            Err(e) => {
                return self.tool_result(id, &format!("Failed to read response: {}", e), true)
            }
        };

        // HTML to text conversion with Regex
        let script_re = regex::Regex::new(r"(?s)<script.*?>.*?</script>").unwrap();
        let style_re = regex::Regex::new(r"(?s)<style.*?>.*?</style>").unwrap();
        let tag_re = regex::Regex::new(r"<[^>]*>").unwrap();

        let no_script = script_re.replace_all(&html, " ");
        let no_style = style_re.replace_all(&no_script, " ");
        let text_content = tag_re.replace_all(&no_style, " ");

        let text = text_content
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");

        // Chunk text with overlap
        let processor = crate::processor::TextProcessor::new();
        let chunks = processor.chunk_text(&text, 1000, 150);

        // Add to vector store
        let store = match self.engine.get_store(namespace) {
            Ok(s) => s,
            Err(e) => return self.tool_result(id, &e.to_string(), true),
        };

        if let Some(ref vector_store) = store.vector_store {
            let mut added_chunks = 0;
            for (i, chunk) in chunks.iter().enumerate() {
                let chunk_uri = format!("{}#chunk-{}", url, i);
                // For MCP ingestion, we just use the chunk URI as the key and metadata URI
                let metadata = serde_json::json!({
                    "uri": chunk_uri,
                    "source_url": url,
                    "type": "web_chunk"
                });
                match vector_store.add(&chunk_uri, chunk, metadata).await {
                    Ok(_) => added_chunks += 1,
                    Err(e) => {
                        eprintln!("Failed to add chunk {}: {}", i, e);
                    }
                }
            }
            let result = IngestToolResult {
                nodes_added: 0,
                edges_added: 0, // Ingest URL technically adds to vector store, no graph edges yet unless reasoned
                message: format!(
                    "Ingested URL: {} ({} chars, {} chunks)",
                    url,
                    text.len(),
                    added_chunks
                ),
            };
            self.serialize_result(id, result)
        } else {
            self.tool_result(id, "Vector store not available", true)
        }
    }

    async fn call_ingest_text(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Map<String, serde_json::Value>,
    ) -> McpResponse {
        let uri = match args.get("uri").and_then(|v| v.as_str()) {
            Some(u) => u,
            None => return self.error_response(id, -32602, "Missing 'uri'"),
        };
        let content = match args.get("content").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return self.error_response(id, -32602, "Missing 'content'"),
        };
        let namespace = args
            .get("namespace")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        // Chunk text with overlap
        let processor = crate::processor::TextProcessor::new();
        let chunks = processor.chunk_text(content, 1000, 150);

        // Add to vector store
        let store = match self.engine.get_store(namespace) {
            Ok(s) => s,
            Err(e) => return self.tool_result(id, &e.to_string(), true),
        };

        if let Some(ref vector_store) = store.vector_store {
            let mut added_chunks = 0;
            for (i, chunk) in chunks.iter().enumerate() {
                let chunk_uri = if chunks.len() > 1 {
                    format!("{}#chunk-{}", uri, i)
                } else {
                    uri.to_string()
                };
                let metadata = serde_json::json!({
                    "uri": uri, // Map back to original URI
                    "chunk_uri": chunk_uri,
                    "type": "text_chunk"
                });
                match vector_store.add(&chunk_uri, chunk, metadata).await {
                    Ok(_) => added_chunks += 1,
                    Err(e) => {
                        eprintln!("Failed to add chunk {}: {}", i, e);
                    }
                }
            }
            let result = IngestToolResult {
                nodes_added: 0,
                edges_added: 0,
                message: format!(
                    "Ingested text: {} ({} chars, {} chunks)",
                    uri,
                    content.len(),
                    added_chunks
                ),
            };
            self.serialize_result(id, result)
        } else {
            self.tool_result(id, "Vector store not available", true)
        }
    }

    async fn call_compact_vectors(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Map<String, serde_json::Value>,
    ) -> McpResponse {
        let namespace = args
            .get("namespace")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        let store = match self.engine.get_store(namespace) {
            Ok(s) => s,
            Err(e) => return self.tool_result(id, &e.to_string(), true),
        };

        if let Some(ref vector_store) = store.vector_store {
            match vector_store.compact() {
                Ok(removed) => {
                    let result = SimpleSuccessResult {
                        success: true,
                        message: format!("Compaction complete: {} stale entries removed", removed),
                    };
                    self.serialize_result(id, result)
                }
                Err(e) => self.tool_result(id, &format!("Compaction error: {}", e), true),
            }
        } else {
            self.tool_result(id, "Vector store not available", true)
        }
    }

    async fn call_vector_stats(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Map<String, serde_json::Value>,
    ) -> McpResponse {
        let namespace = args
            .get("namespace")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        let store = match self.engine.get_store(namespace) {
            Ok(s) => s,
            Err(e) => return self.tool_result(id, &e.to_string(), true),
        };

        if let Some(ref vector_store) = store.vector_store {
            let (active, stale, total) = vector_store.stats();
            let result = StatsToolResult {
                active_vectors: active,
                stale_vectors: stale,
                total_embeddings: total,
            };
            self.serialize_result(id, result)
        } else {
            self.tool_result(id, "Vector store not available", true)
        }
    }

    async fn call_disambiguate(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Map<String, serde_json::Value>,
    ) -> McpResponse {
        let namespace = args
            .get("namespace")
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        let threshold = args
            .get("threshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.8);

        let store = match self.engine.get_store(namespace) {
            Ok(s) => s,
            Err(e) => return self.tool_result(id, &e.to_string(), true),
        };

        // Collect all URIs from the store
        let uri_map = store.uri_to_id.read().unwrap();
        let uris: Vec<String> = uri_map.keys().cloned().collect();
        drop(uri_map);

        let disambiguator = crate::disambiguation::EntityDisambiguator::new(threshold);
        let suggestions = disambiguator.suggest_merges(&uris);

        let items: Vec<DisambiguationItem> = suggestions
            .into_iter()
            .map(|(u1, u2, s)| DisambiguationItem {
                uri1: u1,
                uri2: u2,
                similarity: s,
            })
            .collect();

        let message = if items.is_empty() {
            "No similar entities found above threshold".to_string()
        } else {
            format!("Found {} potential duplicates", items.len())
        };

        let result = DisambiguationResult {
            suggestions: items,
            message,
        };
        self.serialize_result(id, result)
    }

    // Legacy handlers for backward compatibility
    async fn handle_legacy_ingest(&self, request: McpRequest) -> McpResponse {
        let params = match request.params {
            Some(p) => p,
            None => return self.error_response(request.id, -32602, "Invalid params"),
        };

        if let (Some(sub), Some(pred), Some(obj)) = (
            params.get("subject").and_then(|v| v.as_str()),
            params.get("predicate").and_then(|v| v.as_str()),
            params.get("object").and_then(|v| v.as_str()),
        ) {
            let namespace = params
                .get("namespace")
                .and_then(|v| v.as_str())
                .unwrap_or("default");
            let triple = Triple {
                subject: sub.to_string(),
                predicate: pred.to_string(),
                object: obj.to_string(),
                provenance: Some(Provenance {
                    source: "mcp".to_string(),
                    timestamp: "".to_string(),
                    method: "stdio".to_string(),
                }),
                embedding: vec![],
            };

            let req = Self::create_request(IngestRequest {
                triples: vec![triple],
                namespace: namespace.to_string(),
            });

            match self.engine.ingest_triples(req).await {
                Ok(_) => McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(serde_json::to_value("Ingested").unwrap()),
                    error: None,
                },
                Err(e) => self.error_response(request.id, -32000, &e.to_string()),
            }
        } else {
            self.error_response(request.id, -32602, "Invalid params")
        }
    }

    async fn handle_legacy_ingest_file(&self, request: McpRequest) -> McpResponse {
        let params = match request.params {
            Some(p) => p,
            None => {
                return self.error_response(request.id, -32602, "Invalid params: 'path' required")
            }
        };

        if let Some(path) = params.get("path").and_then(|v| v.as_str()) {
            let namespace = params
                .get("namespace")
                .and_then(|v| v.as_str())
                .unwrap_or("default");

            let req = Self::create_request(IngestFileRequest {
                file_path: path.to_string(),
                namespace: namespace.to_string(),
            });

            match self.engine.ingest_file(req).await {
                Ok(resp) => {
                    let inner = resp.into_inner();
                    McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: Some(
                            serde_json::to_value(format!(
                                "Ingested {} triples from {}",
                                inner.edges_added, path
                            ))
                            .unwrap(),
                        ),
                        error: None,
                    }
                }
                Err(e) => self.error_response(request.id, -32000, &e.to_string()),
            }
        } else {
            self.error_response(request.id, -32602, "Invalid params: 'path' required")
        }
    }

    fn serialize_result<T: serde::Serialize>(
        &self,
        id: Option<serde_json::Value>,
        result: T,
    ) -> McpResponse {
        match serde_json::to_string_pretty(&result) {
            Ok(json) => self.tool_result(id, &json, false),
            Err(e) => self.tool_result(id, &format!("Serialization error: {}", e), true),
        }
    }

    async fn call_get_node_degree(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Map<String, serde_json::Value>,
    ) -> McpResponse {
        let uri = match args.get("uri").and_then(|v| v.as_str()) {
            Some(u) => u,
            None => return self.error_response(id, -32602, "Missing 'uri'"),
        };
        let namespace = args
            .get("namespace")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        let store = match self.engine.get_store(namespace) {
            Ok(s) => s,
            Err(e) => return self.tool_result(id, &e.to_string(), true),
        };

        let degree = store.get_degree(uri);

        let result = DegreeResult {
            uri: uri.to_string(),
            degree,
        };

        self.serialize_result(id, result)
    }

    fn error_response(
        &self,
        id: Option<serde_json::Value>,
        code: i32,
        message: &str,
    ) -> McpResponse {
        McpResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(McpError {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }

    fn tool_result(
        &self,
        id: Option<serde_json::Value>,
        text: &str,
        is_error: bool,
    ) -> McpResponse {
        let result = CallToolResult {
            content: vec![Content {
                content_type: "text".to_string(),
                text: text.to_string(),
            }],
            is_error: if is_error { Some(true) } else { None },
        };
        McpResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::to_value(result).unwrap()),
            error: None,
        }
    }
}
