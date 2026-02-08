use std::env;
use synapse_core::server::proto::semantic_engine_server::SemanticEngine; // Correct Trait path
use synapse_core::server::{
    proto::{IngestRequest, NodeRequest, Triple},
    MySemanticEngine,
};
use tonic::Request;

#[tokio::test]
async fn test_graph_traversal_scoring() {
    env::set_var("MOCK_EMBEDDINGS", "true");
    let storage_path = "/tmp/synapse_test_traversal";
    let _ = std::fs::remove_dir_all(storage_path);

    let engine = MySemanticEngine::new(storage_path);
    let namespace = "default"; // Use default for anonymous access

    // Build graph
    // A -> Hub
    // Hub -> X1..X10
    // A -> Leaf
    // Leaf -> Y1

    let mut triples = vec![
        Triple {
            subject: "A".into(),
            predicate: "to".into(),
            object: "Hub".into(),
            ..Default::default()
        },
        Triple {
            subject: "A".into(),
            predicate: "to".into(),
            object: "Leaf".into(),
            ..Default::default()
        },
        Triple {
            subject: "Leaf".into(),
            predicate: "to".into(),
            object: "Y1".into(),
            ..Default::default()
        },
    ];

    for i in 0..10 {
        triples.push(Triple {
            subject: "Hub".into(),
            predicate: "to".into(),
            object: format!("X{}", i),
            ..Default::default()
        });
    }

    let req = Request::new(IngestRequest {
        triples,
        namespace: namespace.into(),
    });
    engine.ingest_triples(req).await.unwrap();

    // Get ID for "A"
    // Use store directly or resolve
    let store = engine.get_store(namespace).unwrap();
    let a_id = store.get_or_create_id("http://synapse.os/A");

    // Test Default Strategy
    let req_default = Request::new(NodeRequest {
        node_id: a_id,
        namespace: namespace.into(),
        direction: "outgoing".into(),
        depth: 1,
        edge_filter: "".into(),
        limit_per_layer: 0,
        scoring_strategy: "default".into(),
        node_type_filter: "".into(),
    });

    let resp_default = engine
        .get_neighbors(req_default)
        .await
        .unwrap()
        .into_inner();
    let neighbors_default = resp_default.neighbors;

    println!("Neighbors: {:?}", neighbors_default);

    let hub_node = neighbors_default
        .iter()
        .find(|n| n.uri.contains("Hub"))
        .unwrap();
    let leaf_node = neighbors_default
        .iter()
        .find(|n| n.uri.contains("Leaf"))
        .unwrap();

    assert!(
        (hub_node.score - leaf_node.score).abs() < 0.001,
        "Default scores should be equal (same depth)"
    );

    // Test Degree Strategy
    let req_degree = Request::new(NodeRequest {
        node_id: a_id,
        namespace: namespace.into(),
        direction: "outgoing".into(),
        depth: 1,
        edge_filter: "".into(),
        limit_per_layer: 0,
        scoring_strategy: "degree".into(),
        node_type_filter: "".into(),
    });

    let resp_degree = engine.get_neighbors(req_degree).await.unwrap().into_inner();
    let neighbors_degree = resp_degree.neighbors;

    let hub_node_d = neighbors_degree
        .iter()
        .find(|n| n.uri.contains("Hub"))
        .unwrap();
    let leaf_node_d = neighbors_degree
        .iter()
        .find(|n| n.uri.contains("Leaf"))
        .unwrap();

    println!(
        "Hub Score: {}, Leaf Score: {}",
        hub_node_d.score, leaf_node_d.score
    );

    assert!(
        hub_node_d.score < leaf_node_d.score,
        "Hub should be penalized"
    );
}
