use std::env;
use synapse_core::server::proto::semantic_engine_server::SemanticEngine;
use synapse_core::server::{
    proto::{IngestRequest, NodeRequest, Triple},
    MySemanticEngine,
};
use tonic::Request;

#[tokio::test]
async fn test_node_type_filter() {
    env::set_var("MOCK_EMBEDDINGS", "true");
    let storage_path = "/tmp/synapse_test_filter";
    let _ = std::fs::remove_dir_all(storage_path);

    let engine = MySemanticEngine::new(storage_path);
    let namespace = "default";

    // Graph:
    // A -> B (Type: Person)
    // A -> C (Type: Bot)
    let triples = vec![
        Triple {
            subject: "A".into(),
            predicate: "knows".into(),
            object: "B".into(),
            ..Default::default()
        },
        Triple {
            subject: "A".into(),
            predicate: "knows".into(),
            object: "C".into(),
            ..Default::default()
        },
        Triple {
            subject: "B".into(),
            predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".into(),
            object: "Person".into(),
            ..Default::default()
        },
        Triple {
            subject: "C".into(),
            predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".into(),
            object: "Bot".into(),
            ..Default::default()
        },
    ];

    let req = Request::new(IngestRequest {
        triples,
        namespace: namespace.into(),
    });
    engine.ingest_triples(req).await.unwrap();

    let store = engine.get_store(namespace).unwrap();
    let a_id = store.get_or_create_id("http://synapse.os/A");

    // Filter for Person
    let req_filter = Request::new(NodeRequest {
        node_id: a_id,
        namespace: namespace.into(),
        direction: "outgoing".into(),
        depth: 1,
        edge_filter: "".into(),
        limit_per_layer: 0,
        scoring_strategy: "default".into(),
        node_type_filter: "http://synapse.os/Person".into(), // B should match
    });

    let resp = engine.get_neighbors(req_filter).await.unwrap().into_inner();
    let neighbors = resp.neighbors;

    println!("Neighbors: {:?}", neighbors);
    assert_eq!(neighbors.len(), 1);
    assert!(neighbors[0].uri.contains("B"));
}

#[tokio::test]
async fn test_auth_read_fail() {
    let storage_path = "/tmp/synapse_test_auth";
    let _ = std::fs::remove_dir_all(storage_path);

    // Set auth tokens
    env::set_var(
        "SYNAPSE_AUTH_TOKENS",
        r#"
    {
        "user_read": {
            "namespaces": ["default"],
            "permissions": {"read": true, "write": false, "delete": false, "reason": false}
        },
        "user_none": {
             "namespaces": ["default"],
             "permissions": {"read": false, "write": false, "delete": false, "reason": false}
        }
    }
    "#,
    );

    // Force reload of auth by creating new engine (auth loads from env in constructor)
    let engine = MySemanticEngine::new(storage_path);
    let namespace = "default";

    // Request with valid read token
    let mut req_good = Request::new(synapse_core::server::proto::EmptyRequest {
        namespace: namespace.into(),
    });
    req_good
        .metadata_mut()
        .insert("authorization", "Bearer user_read".parse().unwrap());

    let res = engine.get_all_triples(req_good).await;
    assert!(res.is_ok(), "Read should succeed with read permission");

    // Request with no permission
    let mut req_bad = Request::new(synapse_core::server::proto::EmptyRequest {
        namespace: namespace.into(),
    });
    req_bad
        .metadata_mut()
        .insert("authorization", "Bearer user_none".parse().unwrap());

    let res = engine.get_all_triples(req_bad).await;
    assert!(res.is_err(), "Read should fail with no permission");
    assert_eq!(res.err().unwrap().code(), tonic::Code::PermissionDenied);
}
