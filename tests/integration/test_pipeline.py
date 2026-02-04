"""Integration test for complete pipeline"""
import pytest
from agents.infrastructure.persistence.graph_client import GraphClient
from agents.infrastructure.persistence.vector_store import VectorStore

def test_full_pipeline():
    """Test the complete processing pipeline"""
    pipeline = SemanticPipeline(
        ontology_paths=["ontology/core.owl", "ontology/agriculture.owl"],
        graph_client=GraphClient(),
        vector_store=VectorStore()
    )
    
    text = "Permaculture is a sustainable farming system"
    result = pipeline.process_text(text)
    
    assert "extracted" in result
    assert "validated" in result
    assert result["validated"] >= 0
