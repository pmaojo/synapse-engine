import pytest
import os
import sys
from typing import List

# Add the project root to sys.path to allow importing agents
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))

from agents.infrastructure.web.client import SemanticEngineClient

@pytest.fixture
def client():
    """Provides a SemanticEngineClient instance."""
    client = SemanticEngineClient(host="localhost", port=50051)
    # We don't call connect() here to allow tests to handle connection failure
    return client

def test_client_connection(client):
    """
    Test that the client can connect to the Rust backend.
    Note: This test requires the Rust server to be running.
    """
    is_connected = client.connect()
    if not is_connected:
        pytest.skip("Rust backend not running at localhost:50051")
    assert client.connected is True

def test_ingest_and_resolve(client):
    """Test ingesting a triple and resolving the node ID."""
    if not client.connect():
        pytest.skip("Rust backend not running")
        
    test_namespace = "test_e2e_namespace"
    
    # Clean up before test
    client.delete_tenant_data(test_namespace)
    
    triples = [
        {"subject": "Synapse", "predicate": "isA", "object": "MemoryEngine"}
    ]
    
    result = client.ingest_triples(triples, namespace=test_namespace)
    assert "error" not in result
    assert result["nodes_added"] >= 2
    assert result["edges_added"] >= 1
    
    node_id = client.resolve_id("Synapse", namespace=test_namespace)
    assert node_id is not None
    
    # Cleanup
    client.delete_tenant_data(test_namespace)
    client.close()

def test_get_all_triples(client):
    """Test retrieving all triples for a namespace."""
    if not client.connect():
        pytest.skip("Rust backend not running")
        
    test_namespace = "test_get_all_namespace"
    client.delete_tenant_data(test_namespace)
    
    triples = [
        {"subject": "Alice", "predicate": "knows", "object": "Bob"},
        {"subject": "Bob", "predicate": "knows", "object": "Charlie"}
    ]
    
    client.ingest_triples(triples, namespace=test_namespace)
    
    all_triples = client.get_all_triples(namespace=test_namespace)
    assert len(all_triples) == 2
    
    subjects = [t["subject"] for t in all_triples]
    assert "Alice" in subjects
    assert "Bob" in subjects
    
    # Cleanup
    client.delete_tenant_data(test_namespace)
    client.close()
