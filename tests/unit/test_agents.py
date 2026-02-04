"""Unit tests for core agents"""
import pytest
from agents.domain.services.ontology import OntologyService

def test_extractor_agent():
    """Test basic extraction"""
    agent = ExtractorAgent()
    input_data = AgentInput(text="A Food Forest is a Permaculture system")
    result = agent.forward(input_data)
    
    assert len(result.triples) > 0
    assert result.confidence > 0

def test_ontology_service():
    """Test ontology loading"""
    service = OntologyService(["ontology/core.owl"])
    
    assert len(service.classes) > 0
    assert len(service.properties) > 0
    
    # Test fuzzy matching
    match = service.fuzzy_match_class("entity")
    assert match is not None
