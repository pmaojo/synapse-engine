"""
End-to-End Tests for Grafoso Semantic System
Tests all major functionality: extraction, reasoning, querying, pipelines
"""
import pytest
import asyncio
from pathlib import Path

# Test fixtures
@pytest.fixture
def rust_client():
    from agents.infrastructure.web.client import get_client
    client = get_client()
    if not client.connected:
        pytest.skip("Rust backend not running")
    return client

@pytest.fixture
def ontology():
    from agents.domain.services.ontology import OntologyService
    return OntologyService(["ontology/core.owl", "ontology/agriculture.owl"])

@pytest.fixture
def sample_csv(tmp_path):
    csv_file = tmp_path / "test_plants.csv"
    csv_file.write_text("""species,family,height
Apple,Rosaceae,5m
Pear,Rosaceae,6m
Cherry,Rosaceae,4m
""")
    return csv_file

# ============================================
# TEST 1: Triple Extraction
# ============================================
def test_triple_extraction(rust_client):
    """Test basic triple extraction and storage"""
    from agents.application.pipelines.datasyn import DataSynPipeline
    
    pipeline = DataSynPipeline()
    
    # Create test file
    import tempfile
    with tempfile.NamedTemporaryFile(mode='w', suffix='.csv', delete=False) as f:
        f.write("species,family\nApple,Rosaceae\n")
        test_file = f.name
    
    result = pipeline.run(test_file)
    
    assert result.success, "Pipeline should succeed"
    assert result.data["triples_extracted"] > 0, "Should extract triples"
    
    # Verify storage in Rust
    all_triples = rust_client.get_all_triples()
    assert len(all_triples) > 0, "Triples should be stored in Rust"

# ============================================
# TEST 2: NL2Cypher Translation
# ============================================
def test_nl2cypher():
    """Test natural language to Cypher translation"""
    from agents.tools.nl2cypher import NL2CypherAgent
    
    agent = NL2CypherAgent()
    
    # Test query (use correct method name)
    cypher = agent._translate_with_patterns("What plants are in Rosaceae?")
    
    assert cypher is not None, "Should generate Cypher query"
    assert "MATCH" in cypher or "match" in cypher.lower(), "Should contain MATCH clause"
    
    # Validate query
    assert agent.validate_cypher(cypher), "Generated Cypher should be valid"

# ============================================
# TEST 3: Cypher Query Execution
# ============================================
def test_cypher_execution(rust_client):
    """Test Cypher query execution on graph"""
    from agents.tools.cypher_executor import CypherExecutor
    
    # First, ensure we have data
    rust_client.ingest_triples([
        ("Apple", "belongsTo", "Rosaceae"),
        ("Pear", "belongsTo", "Rosaceae")
    ])
    
    executor = CypherExecutor()
    
    # Execute query
    cypher = "MATCH (x)-[:belongsTo]->(f) WHERE f = 'Rosaceae' RETURN x"
    results = executor.execute(cypher)
    
    assert "results" in results, "Should return results"
    assert len(results["results"]) >= 2, "Should find Apple and Pear"

# ============================================
# TEST 4: OWL Reasoning
# ============================================
def test_owl_reasoning(ontology):
    """Test OWL inference rules"""
    from agents.tools.owl_reasoner import OWLReasoningAgent
    
    reasoner = OWLReasoningAgent(ontology.graph)
    
    # Input triples
    triples = [
        ("Compost", "improves", "Soil"),
        ("Soil", "supports", "Plants")
    ]
    
    result = reasoner.infer(triples)
    
    assert "inferred_triples" in result, "Should return inferred triples"
    assert result["expansion_ratio"] >= 1.0, "Should expand knowledge"

# ============================================
# TEST 5: Ontology Validation
# ============================================
def test_ontology_validation(ontology):
    """Test ontology-constrained extraction"""
    from agents.tools.ontology_validator import OntologyValidator
    
    validator = OntologyValidator(ontology)
    
    # Test validation (just check it returns proper structure)
    result = validator.validate_triple("Plant", "hasProperty", "Color")
    
    # Check result structure
    assert "valid" in result, "Should return validation result"
    assert "errors" in result, "Should include errors list"
    assert isinstance(result["valid"], bool), "Valid should be boolean"
    assert isinstance(result["errors"], list), "Errors should be list"

# ============================================
# TEST 6: Incremental CSV Processing
# ============================================
def test_incremental_csv_processing(sample_csv, rust_client):
    """Test incremental CSV processing with RAG + OWL"""
    from agents.application.pipelines.datasyn import DataSynPipeline
    
    pipeline = DataSynPipeline()
    result = pipeline.run(str(sample_csv))
    
    assert result.success, "Should process CSV successfully"
    assert result.data["triples_extracted"] >= 3, "Should extract triples from 3 rows"
    
    # Check logs for OWL reasoning trigger
    assert any("OWL" in log for log in result.logs), "Should trigger OWL reasoning"

# ============================================
# TEST 7: LLM Pipeline Manager
# ============================================
def test_llm_orchestrator():
    """Test autonomous tool selection"""
    from agents.application.orchestration.llm_manager import LLMPipelineManager
    
    manager = LLMPipelineManager()
    
    # Test decision making (synchronous)
    tools = manager._decide_with_rules("What improves soil?")
    
    assert len(tools) > 0, "Should select at least one tool"
    assert tools[0].tool.value in ["nl2cypher", "rag_search"], "Should select appropriate tool"

# ============================================
# TEST 8: Human Feedback Loop
# ============================================
def test_air_feedback():
    """Test Agent Lightning AIR system"""
    from agents.infrastructure.ai.air import get_air, RewardSignal
    
    air = get_air()
    air.reset()
    
    # Record events
    air.record_event(RewardSignal.TRIPLE_EXTRACTED, {"count": 5})
    air.record_event(RewardSignal.OWL_CONSISTENT, {"inferences": 3})
    
    summary = air.get_summary()
    
    assert "Total Reward" in summary, "Should track rewards"
    assert air.get_total_reward() > 0, "Should accumulate positive rewards"

# ============================================
# TEST 9: SPARQL Query (if implemented)
# ============================================
@pytest.mark.skip(reason="SPARQL not yet implemented")
def test_sparql_query(ontology):
    """Test SPARQL endpoint"""
    from agents.reasoning.sparql_engine import SPARQLEngine
    
    engine = SPARQLEngine(ontology.graph)
    
    query = """
    SELECT ?s ?p ?o WHERE {
        ?s ?p ?o .
    } LIMIT 10
    """
    
    results = engine.query(query)
    assert len(results) > 0, "Should return SPARQL results"

# ============================================
# TEST 10: Gradio API
# ============================================
def test_gradio_api():
    """Test Gradio API endpoints"""
    import requests
    
    # Assuming Gradio is running on localhost:7860
    base_url = "http://localhost:7860"
    
    try:
        response = requests.get(f"{base_url}/api/", timeout=2)
        assert response.status_code == 200, "API should be accessible"
    except requests.exceptions.ConnectionError:
        pytest.skip("Gradio not running")

# ============================================
# TEST 11: End-to-End Workflow
# ============================================
@pytest.mark.asyncio
async def test_e2e_workflow(sample_csv, rust_client, ontology):
    """Complete end-to-end test: Ingest → Query → Reason"""
    from agents.application.pipelines.datasyn import DataSynPipeline
    from agents.tools.nl2cypher import NL2CypherAgent
    from agents.tools.cypher_executor import CypherExecutor
    from agents.tools.owl_reasoner import OWLReasoningAgent
    
    # Step 1: Ingest CSV
    pipeline = DataSynPipeline()
    ingest_result = pipeline.run(str(sample_csv))
    assert ingest_result.success, "Ingestion should succeed"
    
    # Step 2: Query with NL2Cypher
    nl2cypher = NL2CypherAgent()
    cypher = await nl2cypher.translate("What is in Rosaceae?", use_llm=False)
    
    executor = CypherExecutor()
    query_results = executor.execute(cypher)
    assert len(query_results["results"]) > 0, "Should find results"
    
    # Step 3: Apply OWL reasoning
    reasoner = OWLReasoningAgent(ontology.graph)
    all_triples = rust_client.get_all_triples()
    reasoning_result = reasoner.infer(all_triples[-10:])
    
    assert reasoning_result["expansion_ratio"] >= 1.0, "Should expand knowledge"
    
    print(f"""
    ✅ E2E Test Complete:
    - Ingested: {ingest_result.data['triples_extracted']} triples
    - Queried: {len(query_results['results'])} results
    - Inferred: {len(reasoning_result['inferred_triples'])} new triples
    """)

if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
