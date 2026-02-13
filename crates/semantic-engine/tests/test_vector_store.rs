use std::env;
use synapse_core::store::{IngestTriple, SynapseStore};

#[tokio::test]
async fn test_vector_synchronization() {
    let namespace = "test_vector_sync";
    let storage_path = "/tmp/synapse_test_vector";
    let _ = std::fs::remove_dir_all(storage_path); // Cleanup

    // Enable Mock Embeddings for this test to avoid external API calls
    env::set_var("MOCK_EMBEDDINGS", "true");

    let store = SynapseStore::open(namespace, storage_path).unwrap();

    let triple1 = IngestTriple {
        subject: "http://example.org/dog".to_string(),
        predicate: "http://example.org/isA".to_string(),
        object: "http://example.org/animal".to_string(),
        provenance: None,
    };

    // Ingest first triple
    store.ingest_triples(vec![triple1]).await.unwrap();

    let triple2 = IngestTriple {
        subject: "http://example.org/dog".to_string(),
        predicate: "http://example.org/eats".to_string(),
        object: "http://example.org/food".to_string(),
        provenance: None,
    };

    // Ingest second triple (same subject, should be indexed separately)
    store.ingest_triples(vec![triple2]).await.unwrap();

    // Verify both are in vector store
    let vs = store.vector_store.as_ref().unwrap();
    assert_eq!(vs.len(), 2, "Should have 2 vectors indexed");

    // Verify search works and returns correct URI (Subject)
    // Note: With random embeddings, search is random, but we check structure
    // We limit k to 2 because we only have 2 items, and hnsw might panic if k > N in some versions/configs?
    let results = vs.search("dog", 2).await.unwrap();
    assert!(!results.is_empty());

    // Metadata check
    let first = &results[0];
    // We expect the URI to be exactly what we ingested if it starts with http, OR formatted.
    // In our test input we provided "http://example.org/dog".
    // SynapseStore::ensure_uri implementation: if s.starts_with("http") { s.to_string() }
    // So it should NOT be prefixed with synapse.os.
    // Wait, why did the assertion fail with left "http://example.org/dog" vs right "http://synapse.os/..."?
    // Left is first.uri. Right is expectation.
    // "assertion `left == right` failed"
    // left: "http://example.org/dog"
    // right: "http://synapse.os/http://example.org/dog"
    // So the actual value (left) is "http://example.org/dog".
    // My expectation (right) was wrong in the test code.
    assert_eq!(first.uri, "http://example.org/dog");
    assert!(first.metadata.get("predicate").is_some());

    // Clean up
    env::remove_var("MOCK_EMBEDDINGS");
}
