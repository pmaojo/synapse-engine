use std::env;
use synapse_core::store::{IngestTriple, Provenance, SynapseStore};

#[tokio::test]
async fn test_provenance_persistence() {
    env::set_var("MOCK_EMBEDDINGS", "true");
    let namespace = "test_provenance";
    let storage_path = "/tmp/synapse_test_provenance";
    let _ = std::fs::remove_dir_all(storage_path); // Cleanup

    let store = SynapseStore::open(namespace, storage_path).unwrap();

    let prov = Provenance {
        source: "test_source".to_string(),
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        method: "manual".to_string(),
    };

    let triple = IngestTriple {
        subject: "http://example.org/alice".to_string(),
        predicate: "http://example.org/knows".to_string(),
        object: "http://example.org/bob".to_string(),
        provenance: Some(prov.clone()),
    };

    let (nodes, _edges) = store.ingest_triples(vec![triple]).await.unwrap();
    // In our implementation, we add edges. We don't distinguish node addition in the return value currently (always returns added, 0).
    assert_eq!(nodes, 1);

    // Verify triples are in a named graph
    // SPARQL: SELECT ?g WHERE { GRAPH ?g { <http://example.org/alice> <http://example.org/knows> <http://example.org/bob> } }

    let query = "SELECT ?g WHERE { GRAPH ?g { <http://example.org/alice> <http://example.org/knows> <http://example.org/bob> } }";
    let result_json = store.query_sparql(query).unwrap();
    println!("SPARQL Result: {}", result_json);

    assert!(result_json.contains("urn:batch:"));

    // Verify provenance metadata in default graph
    // SPARQL: SELECT ?s WHERE { ?s <http://www.w3.org/ns/prov#wasDerivedFrom> "test_source" }
    // Note: Literal matching in SPARQL might need type or exact string.
    let query_meta =
        "SELECT ?s WHERE { ?s <http://www.w3.org/ns/prov#wasDerivedFrom> \"test_source\" }";
    let result_meta = store.query_sparql(query_meta).unwrap();
    assert_ne!(result_meta, "[]");
}
