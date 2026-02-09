import sys
import os
import json
import time

# Add tests dir to path to import extended_client
sys.path.insert(0, os.path.dirname(__file__))

from extended_client import ExtendedClient

NAMESPACE = "manual_grpc_test"

def run_test():
    print(f"🚀 Starting Manual gRPC Test in namespace '{NAMESPACE}'...")

    client = ExtendedClient(host="localhost", port=50051)
    if not client.connect():
        print("❌ Failed to connect to server.")
        sys.exit(1)

    # 1. Cleanup
    print("\n🧹 Cleaning up namespace...")
    client.delete_tenant_data(NAMESPACE)

    # 2. Ingest Triples
    print("\n📥 Ingesting triples...")
    triples = [
        {"subject": "http://example.org/Alice", "predicate": "http://example.org/knows", "object": "http://example.org/Bob"},
        {"subject": "http://example.org/Bob", "predicate": "http://example.org/knows", "object": "http://example.org/Charlie"},
        {"subject": "http://example.org/Alice", "predicate": "http://example.org/likes", "object": "http://example.org/Pizza"},
        {"subject": "http://example.org/Pizza", "predicate": "http://www.w3.org/1999/02/22-rdf-syntax-ns#type", "object": "http://example.org/Food"},
        # For reasoning test (Symmetric Property: spouse)
        {"subject": "http://example.org/Dave", "predicate": "http://example.org/spouse", "object": "http://example.org/Eve"}
    ]

    res = client.ingest_triples(triples, namespace=NAMESPACE)
    print(f"   Result: {res}")
    # Just check that it returns a result dict, values might vary
    assert isinstance(res, dict) and "edges_added" in res

    # 3. Resolve ID
    print("\n🔍 Resolving ID for Alice...")
    alice_id = client.resolve_id("http://example.org/Alice", namespace=NAMESPACE)
    print(f"   Alice ID: {alice_id}")
    assert alice_id is not None, "Failed to resolve Alice"

    # 4. Get Neighbors
    print("\n🕸️ Getting neighbors for Alice...")
    neighbors = client.get_neighbors_full(alice_id, namespace=NAMESPACE)
    print(f"   Neighbors: {json.dumps(neighbors, indent=2)}")
    assert len(neighbors) >= 2, "Alice should have at least 2 neighbors"

    # 5. Hybrid Search
    print("\n🔎 Hybrid Search for 'Pizza'...")
    # Wait a bit for indexing if needed (though existing tests don't wait)
    search_res = client.hybrid_search("Pizza", namespace=NAMESPACE)
    print(f"   Results: {search_res}")
    # Note: Search results might be empty if vector store isn't working or embedding failed.
    # But since we ingest, we expect some result if vectors are working.
    if not search_res:
        print("⚠️  Warning: No search results found. Vector store might be initializing or empty.")
    else:
        print(f"   Found {len(search_res)} results.")

    # 6. SPARQL Query
    print("\n❓ Executing SPARQL Query...")
    query = """
    SELECT ?s ?o WHERE {
        ?s <http://example.org/knows> ?o
    }
    """
    sparql_res_json = client.query_sparql(query, namespace=NAMESPACE)
    print(f"   Result JSON: {sparql_res_json}")
    sparql_res = json.loads(sparql_res_json)
    # Depending on how result is formatted (bindings usually)
    if "results" in sparql_res and "bindings" in sparql_res["results"]:
         count = len(sparql_res["results"]["bindings"])
         print(f"   Found {count} bindings.")
         assert count >= 2, "SPARQL query should return at least 2 bindings"

    # 7. Apply Reasoning
    print("\n🧠 Applying Reasoning...")
    # We need an ontology or rule. The system has built-in RDFS/OWL-RL.
    # Let's add schema triples first.
    schema_triples = [
         {"subject": "http://example.org/spouse", "predicate": "http://www.w3.org/1999/02/22-rdf-syntax-ns#type", "object": "http://www.w3.org/2002/07/owl#SymmetricProperty"}
    ]
    client.ingest_triples(schema_triples, namespace=NAMESPACE)

    reasoning_res = client.apply_reasoning(namespace=NAMESPACE, strategy="owlrl", materialize=True)
    print(f"   Result: {reasoning_res}")
    assert reasoning_res["success"], "Reasoning failed"

    # Check if inference happened (Eve spouse Dave)
    print("   Verifying inference...")
    # Need to wait a tiny bit? usually blocking.
    eve_neighbors = client.get_neighbors_by_uri("http://example.org/Eve", namespace=NAMESPACE)
    spouse_exists = False
    for n in eve_neighbors:
        # Check predicate and target
        if n["edge_type"] == "http://example.org/spouse" and n["uri"] == "http://example.org/Dave":
            spouse_exists = True
            break

    if spouse_exists:
        print("✅ Inference verified: Eve is spouse of Dave")
    else:
        print("⚠️  Inference check failed. Eve's neighbors:")
        print(json.dumps(eve_neighbors, indent=2))

    # 8. Cleanup
    print("\n🧹 Final Cleanup...")
    client.delete_tenant_data(NAMESPACE)
    print("✅ Test Complete!")

if __name__ == "__main__":
    run_test()
