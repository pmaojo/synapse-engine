use crate::server::MySemanticEngine;
use crate::server::proto::semantic_engine_server::SemanticEngine;
use crate::server::proto::{IngestRequest, IngestFileRequest, Triple, Provenance};
use crate::mcp_types::{McpRequest, McpResponse};
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

            if let Ok(request) = serde_json::from_str::<McpRequest>(&line) {
                let response = self.handle_request(request).await;
                let response_json = serde_json::to_string(&response)? + "\n";
                writer.write_all(response_json.as_bytes()).await?;
            }
        }

        Ok(())
    }

    async fn handle_request(&self, request: McpRequest) -> McpResponse {
        match request.method.as_str() {
            "ingest" => {
                if let Some(params) = request.params {
                    if let (Some(sub), Some(pred), Some(obj)) = (
                        params.get("subject").and_then(|v| v.as_str()),
                        params.get("predicate").and_then(|v| v.as_str()),
                        params.get("object").and_then(|v| v.as_str()),
                    ) {
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

                        let engine = self.engine.clone();
                        let ingest_request = Request::new(IngestRequest {
                            triples: vec![triple],
                            namespace: "default".to_string(),
                        });

                        match engine.ingest_triples(ingest_request).await {
                            Ok(_) => {
                                return McpResponse {
                                    jsonrpc: "2.0".to_string(),
                                    id: request.id,
                                    result: Some(serde_json::to_value("Ingested").unwrap()),
                                    error: None,
                                };
                            }
                            Err(e) => {
                                return McpResponse {
                                    jsonrpc: "2.0".to_string(),
                                    id: request.id,
                                    result: None,
                                    error: Some(crate::mcp_types::McpError {
                                        code: -32000,
                                        message: e.to_string(),
                                        data: None,
                                    }),
                                };
                            }
                        }
                    }
                }
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(crate::mcp_types::McpError {
                        code: -32602,
                        message: "Invalid params".to_string(),
                        data: None,
                    }),
                }
            }
            "ingest_file" => {
                if let Some(params) = request.params {
                    if let Some(path) = params.get("path").and_then(|v| v.as_str()) {
                        let namespace = params.get("namespace")
                            .and_then(|v| v.as_str())
                            .unwrap_or("default");

                        let engine = self.engine.clone();
                        let ingest_request = Request::new(IngestFileRequest {
                            file_path: path.to_string(),
                            namespace: namespace.to_string(),
                        });

                        match engine.ingest_file(ingest_request).await {
                            Ok(resp) => {
                                let inner = resp.into_inner();
                                return McpResponse {
                                    jsonrpc: "2.0".to_string(),
                                    id: request.id,
                                    result: Some(serde_json::to_value(format!(
                                        "Ingested {} triples from {}", 
                                        inner.edges_added, path
                                    )).unwrap()),
                                    error: None,
                                };
                            }
                            Err(e) => {
                                return McpResponse {
                                    jsonrpc: "2.0".to_string(),
                                    id: request.id,
                                    result: None,
                                    error: Some(crate::mcp_types::McpError {
                                        code: -32000,
                                        message: e.to_string(),
                                        data: None,
                                    }),
                                };
                            }
                        }
                    }
                }
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(crate::mcp_types::McpError {
                        code: -32602,
                        message: "Invalid params: 'path' required".to_string(),
                        data: None,
                    }),
                }
            }
            _ => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(crate::mcp_types::McpError {
                    code: -32601,
                    message: "Method not found".to_string(),
                    data: None,
                }),
            },
        }
    }
}
