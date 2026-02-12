use std::env;
use synapse_core::server::proto::semantic_engine_server::SemanticEngine;
use synapse_core::server::proto::{IngestRequest, NodeRequest, Triple};
use synapse_core::server::MySemanticEngine;
use tonic::Request;

#[tokio::test]
async fn test_get_neighbors_deterministic_scoring() {
    env::set_var("MOCK_EMBEDDINGS", "true");
    env::set_var("SYNAPSE_AUTH_TOKENS", "{\"test-token\": [\"*\"]}");
    let storage_path = "/tmp/synapse_test_neighbors";
    let _ = std::fs::remove_dir_all(storage_path);

    let engine = MySemanticEngine::new(storage_path);

    // Setup Graph: A -> B, A -> C, C -> D
    let triples = vec![
        Triple {
            subject: "http://a".into(),
            predicate: "http://p".into(),
            object: "http://b".into(),
            provenance: None,
            embedding: vec![],
        },
        Triple {
            subject: "http://a".into(),
            predicate: "http://p".into(),
            object: "http://c".into(),
            provenance: None,
            embedding: vec![],
        },
        Triple {
            subject: "http://c".into(),
            predicate: "http://p".into(),
            object: "http://d".into(),
            provenance: None,
            embedding: vec![],
        },
    ];

    let mut ingest_req = Request::new(IngestRequest {
        namespace: "test".into(),
        triples,
    });
    ingest_req
        .metadata_mut()
        .insert("authorization", "Bearer test-token".parse().unwrap());
    engine.ingest_triples(ingest_req).await.unwrap();

    // 1. Resolve ID for "http://a"
    let store = engine.get_store("test").unwrap();
    let id_a = store.get_or_create_id("http://a");

    // 2. Query Neighbors of A with strategy "degree"
    let req = NodeRequest {
        namespace: "test".into(),
        node_id: id_a,
        depth: 1,
        direction: "outgoing".into(),
        scoring_strategy: "degree".into(),
        edge_filter: "".into(),
        node_type_filter: "".into(),
        limit_per_layer: 0,
    };

    let mut req_wrapped = Request::new(req);
    req_wrapped
        .metadata_mut()
        .insert("authorization", "Bearer test-token".parse().unwrap());
    let resp = engine
        .get_neighbors(req_wrapped)
        .await
        .unwrap()
        .into_inner();

    // B should be first (lower degree than C which connects to D)
    // Wait, let's verify degrees:
    // B: 1 (incoming from A)
    // C: 2 (incoming from A, outgoing to D)
    // Current logic penalizes high degree. So B should have higher score than C.

    assert_eq!(resp.neighbors.len(), 2);
    for n in &resp.neighbors {
        println!("Found neighbor: {} with score {}", n.uri, n.score);
    }
    let n_b = resp
        .neighbors
        .iter()
        .find(|n| n.uri.contains("http://b"))
        .unwrap();
    let n_c = resp
        .neighbors
        .iter()
        .find(|n| n.uri.contains("http://c"))
        .unwrap();

    assert!(
        n_b.score > n_c.score,
        "B (degree 1) should have higher score than C (degree 2). B: {}, C: {}",
        n_b.score,
        n_c.score
    );

    // 3. Query with depth 2
    let req_depth = NodeRequest {
        namespace: "test".into(),
        node_id: id_a,
        depth: 2,
        direction: "outgoing".into(),
        scoring_strategy: "path".into(),
        edge_filter: "".into(),
        node_type_filter: "".into(),
        limit_per_layer: 0,
    };

    let mut req_depth_wrapped = Request::new(req_depth);
    req_depth_wrapped
        .metadata_mut()
        .insert("authorization", "Bearer test-token".parse().unwrap());
    let resp_depth = engine
        .get_neighbors(req_depth_wrapped)
        .await
        .unwrap()
        .into_inner();

    // A -> B (depth 1)
    // A -> C (depth 1)
    // A -> C -> D (depth 2)
    // Note: The neighbor finding logic in BFS might visit nodes but filter duplicates.
    // If D is reached via C, it should be included.

    // Debug output
    for n in &resp_depth.neighbors {
        println!("Depth 2 neighbor: {} (depth {})", n.uri, n.depth);
    }

    assert_eq!(resp_depth.neighbors.len(), 3);
    let n_d = resp_depth
        .neighbors
        .iter()
        .find(|n| n.uri.contains("http://d"))
        .unwrap();
    assert_eq!(n_d.depth, 2);
    assert!(
        n_b.score > n_d.score,
        "Depth 1 node should have higher score than depth 2 node"
    );
}
