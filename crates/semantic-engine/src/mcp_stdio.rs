use crate::server::semantic_engine::semantic_engine_server::SemanticEngine;
use crate::server::MySemanticEngine;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tokio::io::{stdin, AsyncBufReadExt, BufReader};

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    _jsonrpc: String,
    method: String,
    params: Option<serde_json::Value>,
    id: Option<serde_json::Value>,
}

pub async fn run_mcp_stdio(
    engine: Arc<MySemanticEngine>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = BufReader::new(stdin()).lines();

    while let Some(line) = reader.next_line().await? {
        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(_) => continue,
        };

        let response = match request.method.as_str() {
            "initialize" => json!({
                "jsonrpc": "2.0",
                "id": request.id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "Synapse-MCP",
                        "version": "0.1.0"
                    }
                }
            }),
            "tools/list" => json!({
                "jsonrpc": "2.0",
                "id": request.id,
                "result": {
                    "tools": [
                        {
                            "name": "query_graph",
                            "description": "Busca triples en el grafo de conocimiento",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "namespace": { "type": "string", "default": "robin_os" }
                                }
                            }
                        },
                        {
                            "name": "ingest_triple",
                            "description": "Añade un nuevo triple (sujeto, predicado, objeto) al grafo",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "subject": { "type": "string" },
                                    "predicate": { "type": "string" },
                                    "object": { "type": "string" },
                                    "namespace": { "type": "string", "default": "robin_os" }
                                },
                                "required": ["subject", "predicate", "object"]
                            }
                        },
                        {
                            "name": "query_sparql",
                            "description": "Ejecuta una consulta SPARQL sobre el grafo",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "query": { "type": "string" },
                                    "namespace": { "type": "string", "default": "robin_os" }
                                },
                                "required": ["query"]
                            }
                        }
                    ]
                }
            }),
            "tools/call" => {
                let params = request.params.unwrap_or(json!({}));
                let name = params["name"].as_str().unwrap_or("");
                let args = &params["arguments"];

                match name {
                    "query_graph" => {
                        let namespace = args["namespace"].as_str().unwrap_or("robin_os");
                        let triples = engine
                            .get_all_triples(tonic::Request::new(
                                crate::server::semantic_engine::EmptyRequest {
                                    namespace: namespace.to_string(),
                                },
                            ))
                            .await?;

                        let triples_text = triples
                            .into_inner()
                            .triples
                            .into_iter()
                            .map(|t| format!("({}, {}, {})", t.subject, t.predicate, t.object))
                            .collect::<Vec<_>>()
                            .join("\n");

                        json!({
                            "jsonrpc": "2.0",
                            "id": request.id,
                            "result": {
                                "content": [{ "type": "text", "text": triples_text }]
                            }
                        })
                    }
                    "ingest_triple" => {
                        let sub = args["subject"].as_str().unwrap_or("");
                        let pred = args["predicate"].as_str().unwrap_or("");
                        let obj = args["object"].as_str().unwrap_or("");
                        let namespace = args["namespace"].as_str().unwrap_or("robin_os");

                        let triple = crate::server::semantic_engine::Triple {
                            subject: sub.to_string(),
                            predicate: pred.to_string(),
                            object: obj.to_string(),
                            provenance: None,
                        };

                        engine
                            .ingest_triples(tonic::Request::new(
                                crate::server::semantic_engine::IngestRequest {
                                    triples: vec![triple],
                                    namespace: namespace.to_string(),
                                },
                            ))
                            .await?;

                        json!({
                            "jsonrpc": "2.0",
                            "id": request.id,
                            "result": {
                                "content": [{ "type": "text", "text": format!("Triple ({}, {}, {}) ingerido correctamente en Synapse", sub, pred, obj) }]
                            }
                        })
                    }
                    "query_sparql" => {
                        let query = args["query"].as_str().unwrap_or("");
                        let namespace = args["namespace"].as_str().unwrap_or("robin_os");

                        let res = engine
                            .query_sparql(tonic::Request::new(
                                crate::server::semantic_engine::SparqlRequest {
                                    query: query.to_string(),
                                    namespace: namespace.to_string(),
                                },
                            ))
                            .await?;

                        json!({
                            "jsonrpc": "2.0",
                            "id": request.id,
                            "result": {
                                "content": [{ "type": "text", "text": res.into_inner().results_json }]
                            }
                        })
                    }
                    _ => json!({
                        "jsonrpc": "2.0",
                        "id": request.id,
                        "error": { "code": -32601, "message": "Tool not found" }
                    }),
                }
            }
            _ => json!({
                "jsonrpc": "2.0",
                "id": request.id,
                "result": {}
            }),
        };

        println!("{}", serde_json::to_string(&response)?);
    }

    Ok(())
}
