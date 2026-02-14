use serde_json::json;
use std::env;
use std::sync::Arc;
use synapse_core::mcp_stdio::McpStdioServer;
use synapse_core::mcp_types::{DegreeResult, IngestToolResult, McpRequest};
use synapse_core::server::MySemanticEngine;

#[tokio::test]
async fn test_mcp_integration() {
    env::set_var("MOCK_EMBEDDINGS", "true");
    let storage_path = "/tmp/synapse_test_mcp";
    let _ = std::fs::remove_dir_all(storage_path);

    let engine = Arc::new(MySemanticEngine::new(storage_path));
    let server = McpStdioServer::new(engine);

    // 1. Ingest Triples
    let req_ingest = McpRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "tools/call".into(),
        params: Some(json!({
            "name": "ingest_triples",
            "arguments": {
                "namespace": "default",
                "triples": [
                    { "subject": "http://a", "predicate": "http://p", "object": "http://b" },
                    { "subject": "http://b", "predicate": "http://p", "object": "http://c" },
                    { "subject": "http://b", "predicate": "http://p", "object": "http://d" }
                ]
            }
        })),
    };

    let resp_ingest = server.handle_request(req_ingest).await;
    if let Some(err) = &resp_ingest.error {
        panic!("Ingest failed: {:?}", err);
    }

    let res_json_str = resp_ingest.result.as_ref().unwrap().get("content").unwrap()[0]
        .get("text")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
    println!("Ingest Response: {}", res_json_str);

    let ingest_result: IngestToolResult =
        serde_json::from_str(&res_json_str).expect("Failed to deserialize IngestToolResult");
    assert_eq!(ingest_result.edges_added, 3);

    // 2. Get Degree of B
    // B connects to C and D (outgoing 2) and from A (incoming 1). Degree = 3.
    // IngestTriples with "http://..." URIs preserves them as is (SynapseStore::ensure_uri).

    let req_degree = McpRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "tools/call".into(),
        params: Some(json!({
            "name": "get_node_degree",
            "arguments": {
                "namespace": "default",
                "uri": "http://b"
            }
        })),
    };

    let resp_degree = server.handle_request(req_degree).await;
    if let Some(err) = &resp_degree.error {
        panic!("Get Degree failed: {:?}", err);
    }

    let degree_json_str = resp_degree.result.as_ref().unwrap().get("content").unwrap()[0]
        .get("text")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
    println!("Degree Response: {}", degree_json_str);

    let degree_result: DegreeResult =
        serde_json::from_str(&degree_json_str).expect("Failed to deserialize DegreeResult");
    assert_eq!(degree_result.degree, 3);
}
