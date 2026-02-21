use std::env;
use synapse_core::store::{IngestTriple, SynapseStore};

#[tokio::test]
async fn test_literal_ingest() {
    env::set_var("MOCK_EMBEDDINGS", "true");
    let namespace = "test_literal";
    let storage_path = "/tmp/synapse_test_literal";
    let _ = std::fs::remove_dir_all(storage_path); // Cleanup

    let store = SynapseStore::open(namespace, storage_path).unwrap();

    let triple = IngestTriple {
        subject: "http://example.org/alice".to_string(),
        predicate: "http://example.org/name".to_string(),
        object: "\"Alice\"".to_string(), // Quoted string
        provenance: None,
    };

    store.ingest_triples(vec![triple]).await.unwrap();

    // Query for literal
    // We expect ?o to be "Alice" (literal string)
    let query = "SELECT ?o WHERE { <http://example.org/alice> <http://example.org/name> ?o . FILTER(isLiteral(?o)) }";
    let result_json = store.query_sparql(query).unwrap();
    println!("SPARQL Result: {}", result_json);

    // If it was ingested as a URI, result_json will be "[]"
    assert!(result_json.contains("Alice"), "Expected literal 'Alice' in results, got: {}", result_json);

    // Also verify it is NOT a URI
    let query_uri = "SELECT ?o WHERE { <http://example.org/alice> <http://example.org/name> ?o . FILTER(isIRI(?o)) }";
    let result_uri = store.query_sparql(query_uri).unwrap();
    assert_eq!(result_uri, "[]", "Expected no URI results, got: {}", result_uri);
}
