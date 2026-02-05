use crate::server::MySemanticEngine;
use crate::server::proto::semantic_engine_server::SemanticEngine;
use crate::server::proto::{
    IngestRequest, IngestFileRequest, Triple, Provenance, 
    SparqlRequest, HybridSearchRequest, ReasoningRequest,
    SearchMode, ReasoningStrategy,
};
use crate::mcp_types::{McpRequest, McpResponse, McpError, Tool, ListToolsResult, CallToolResult, Content};
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

    fn get_tools() -> Vec<Tool> {
        vec![
            Tool {
                name: "ingest_triples".to_string(),
                description: Some("Ingest one or more RDF triples into the knowledge graph".to_string()),
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
                description: Some("Ingest a CSV or Markdown file into the knowledge graph".to_string()),
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
                description: Some("Apply RDFS or OWL-RL reasoning to infer new triples".to_string()),
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
                description: Some("Get neighboring nodes connected to a given URI in the graph".to_string()),
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
                description: Some("List all triples in a namespace (useful for debugging/exploration)".to_string()),
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
                description: Some("Scrape a web page and add its content to the vector store for RAG retrieval".to_string()),
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
                description: Some("Add arbitrary text content to the vector store for RAG retrieval".to_string()),
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
        ]
    }

    async fn handle_request(&self, request: McpRequest) -> McpResponse {
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
                            "version": "0.2.0"
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
                let result = ListToolsResult { tools: Self::get_tools() };
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(serde_json::to_value(result).unwrap()),
                    error: None,
                }
            }
            "tools/call" => {
                self.handle_tool_call(request).await
            }
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

    async fn handle_tool_call(&self, request: McpRequest) -> McpResponse {
        let params = match request.params {
            Some(p) => p,
            None => return self.error_response(request.id, -32602, "Missing params"),
        };

        let tool_name = match params.get("name").and_then(|v| v.as_str()) {
            Some(n) => n,
            None => return self.error_response(request.id, -32602, "Missing tool name"),
        };

        let arguments = params.get("arguments")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

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
            _ => self.error_response(request.id, -32602, &format!("Unknown tool: {}", tool_name)),
        }
    }

    async fn call_ingest_triples(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Map<String, serde_json::Value>,
    ) -> McpResponse {
        let namespace = args.get("namespace").and_then(|v| v.as_str()).unwrap_or("default");
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

        let req = Request::new(IngestRequest {
            triples,
            namespace: namespace.to_string(),
        });

        match self.engine.ingest_triples(req).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                self.tool_result(id, &format!("Ingested {} triples", inner.edges_added), false)
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
        let namespace = args.get("namespace").and_then(|v| v.as_str()).unwrap_or("default");

        let req = Request::new(IngestFileRequest {
            file_path: path.to_string(),
            namespace: namespace.to_string(),
        });

        match self.engine.ingest_file(req).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                self.tool_result(id, &format!("Ingested {} triples from {}", inner.edges_added, path), false)
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
        let namespace = args.get("namespace").and_then(|v| v.as_str()).unwrap_or("default");

        let req = Request::new(SparqlRequest {
            query: query.to_string(),
            namespace: namespace.to_string(),
        });

        match self.engine.query_sparql(req).await {
            Ok(resp) => {
                self.tool_result(id, &resp.into_inner().results_json, false)
            }
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
        let namespace = args.get("namespace").and_then(|v| v.as_str()).unwrap_or("default");
        let vector_k = args.get("vector_k").and_then(|v| v.as_u64()).unwrap_or(10) as u32;
        let graph_depth = args.get("graph_depth").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as u32;

        let req = Request::new(HybridSearchRequest {
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
                // Manually serialize since proto SearchResult doesn't derive Serialize
                let json_results: Vec<serde_json::Value> = results.iter().map(|r| {
                    serde_json::json!({
                        "node_id": r.node_id,
                        "score": r.score,
                        "content": r.content,
                        "uri": r.uri
                    })
                }).collect();
                let json = serde_json::to_string_pretty(&json_results).unwrap_or_default();
                self.tool_result(id, &json, false)
            }
            Err(e) => self.tool_result(id, &e.to_string(), true),
        }
    }

    async fn call_apply_reasoning(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Map<String, serde_json::Value>,
    ) -> McpResponse {
        let namespace = args.get("namespace").and_then(|v| v.as_str()).unwrap_or("default");
        let strategy_str = args.get("strategy").and_then(|v| v.as_str()).unwrap_or("rdfs");
        let materialize = args.get("materialize").and_then(|v| v.as_bool()).unwrap_or(false);

        let strategy = match strategy_str.to_lowercase().as_str() {
            "owlrl" | "owl-rl" => ReasoningStrategy::Owlrl as i32,
            _ => ReasoningStrategy::Rdfs as i32,
        };

        let req = Request::new(ReasoningRequest {
            namespace: namespace.to_string(),
            strategy,
            materialize,
        });

        match self.engine.apply_reasoning(req).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                self.tool_result(id, &inner.message, !inner.success)
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
        let namespace = args.get("namespace").and_then(|v| v.as_str()).unwrap_or("default");
        let direction = args.get("direction").and_then(|v| v.as_str()).unwrap_or("outgoing");

        let store = match self.engine.get_store(namespace) {
            Ok(s) => s,
            Err(e) => return self.tool_result(id, &e.to_string(), true),
        };

        let mut neighbors = Vec::new();

        // Query outgoing edges (URI as subject)
        if direction == "outgoing" || direction == "both" {
            if let Ok(subj) = oxigraph::model::NamedNodeRef::new(uri) {
                for quad in store.store.quads_for_pattern(Some(subj.into()), None, None, None) {
                    if let Ok(q) = quad {
                        neighbors.push(serde_json::json!({
                            "direction": "outgoing",
                            "predicate": q.predicate.to_string(),
                            "target": q.object.to_string()
                        }));
                    }
                }
            }
        }

        // Query incoming edges (URI as object)
        if direction == "incoming" || direction == "both" {
            if let Ok(obj) = oxigraph::model::NamedNodeRef::new(uri) {
                for quad in store.store.quads_for_pattern(None, None, Some(obj.into()), None) {
                    if let Ok(q) = quad {
                        neighbors.push(serde_json::json!({
                            "direction": "incoming",
                            "predicate": q.predicate.to_string(),
                            "source": q.subject.to_string()
                        }));
                    }
                }
            }
        }

        let json = serde_json::to_string_pretty(&neighbors).unwrap_or_default();
        self.tool_result(id, &json, false)
    }

    async fn call_list_triples(
        &self,
        id: Option<serde_json::Value>,
        args: &serde_json::Map<String, serde_json::Value>,
    ) -> McpResponse {
        let namespace = args.get("namespace").and_then(|v| v.as_str()).unwrap_or("default");
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as usize;

        let store = match self.engine.get_store(namespace) {
            Ok(s) => s,
            Err(e) => return self.tool_result(id, &e.to_string(), true),
        };

        let mut triples = Vec::new();
        for quad in store.store.iter().take(limit) {
            if let Ok(q) = quad {
                triples.push(serde_json::json!({
                    "subject": q.subject.to_string(),
                    "predicate": q.predicate.to_string(),
                    "object": q.object.to_string()
                }));
            }
        }

        let json = serde_json::to_string_pretty(&triples).unwrap_or_default();
        self.tool_result(id, &json, false)
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

        let req = Request::new(crate::server::proto::EmptyRequest {
            namespace: namespace.to_string(),
        });

        match self.engine.delete_namespace_data(req).await {
            Ok(resp) => {
                let inner = resp.into_inner();
                self.tool_result(id, &inner.message, !inner.success)
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
        let namespace = args.get("namespace").and_then(|v| v.as_str()).unwrap_or("default");

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
            Err(e) => return self.tool_result(id, &format!("Failed to read response: {}", e), true),
        };

        // Simple HTML to text conversion (strip tags)
        let text = html
            .split('<')
            .filter_map(|s| s.split('>').nth(1))
            .collect::<Vec<_>>()
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");

        // Add to vector store
        let store = match self.engine.get_store(namespace) {
            Ok(s) => s,
            Err(e) => return self.tool_result(id, &e.to_string(), true),
        };

        if let Some(ref vector_store) = store.vector_store {
            match vector_store.add(url, &text).await {
                Ok(_) => self.tool_result(id, &format!("Ingested URL: {} ({} chars)", url, text.len()), false),
                Err(e) => self.tool_result(id, &format!("Vector store error: {}", e), true),
            }
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
        let namespace = args.get("namespace").and_then(|v| v.as_str()).unwrap_or("default");

        // Add to vector store
        let store = match self.engine.get_store(namespace) {
            Ok(s) => s,
            Err(e) => return self.tool_result(id, &e.to_string(), true),
        };

        if let Some(ref vector_store) = store.vector_store {
            match vector_store.add(uri, content).await {
                Ok(_) => self.tool_result(id, &format!("Ingested text: {} ({} chars)", uri, content.len()), false),
                Err(e) => self.tool_result(id, &format!("Vector store error: {}", e), true),
            }
        } else {
            self.tool_result(id, "Vector store not available", true)
        }
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
            let namespace = params.get("namespace").and_then(|v| v.as_str()).unwrap_or("default");
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

            let req = Request::new(IngestRequest {
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
            None => return self.error_response(request.id, -32602, "Invalid params: 'path' required"),
        };

        if let Some(path) = params.get("path").and_then(|v| v.as_str()) {
            let namespace = params.get("namespace").and_then(|v| v.as_str()).unwrap_or("default");

            let req = Request::new(IngestFileRequest {
                file_path: path.to_string(),
                namespace: namespace.to_string(),
            });

            match self.engine.ingest_file(req).await {
                Ok(resp) => {
                    let inner = resp.into_inner();
                    McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: Some(serde_json::to_value(format!(
                            "Ingested {} triples from {}",
                            inner.edges_added, path
                        )).unwrap()),
                        error: None,
                    }
                }
                Err(e) => self.error_response(request.id, -32000, &e.to_string()),
            }
        } else {
            self.error_response(request.id, -32602, "Invalid params: 'path' required")
        }
    }

    fn error_response(&self, id: Option<serde_json::Value>, code: i32, message: &str) -> McpResponse {
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

    fn tool_result(&self, id: Option<serde_json::Value>, text: &str, is_error: bool) -> McpResponse {
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
