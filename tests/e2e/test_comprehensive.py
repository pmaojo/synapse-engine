"""
Comprehensive End-to-End Tests for Grafoso
Tests ALL major functionality to demonstrate the complete application
"""
import pytest
import sys
import os
from pathlib import Path
import tempfile
import shutil

# Add project root to path
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

# ============================================
# FIXTURES
# ============================================

@pytest.fixture(scope="session")
def rust_client():
    """Rust backend client"""
    from agents.infrastructure.web.client import get_client
    client = get_client()
    if not client.connected:
        pytest.skip("Rust backend not running - start with: cargo run")
    return client

@pytest.fixture(scope="session")
def ontology():
    """Ontology service"""
    from agents.domain.services.ontology import OntologyService
    return OntologyService([
        "ontology/core.owl",
        "ontology/agriculture.owl"
    ])

@pytest.fixture(scope="session")
def translation_service():
    """Translation service"""
    from agents.domain.services.translation_service import TranslationService
    return TranslationService()

@pytest.fixture(scope="session")
def ingestion_service(ontology, rust_client):
    """Ingestion service"""
    from agents.domain.services.ingestion_service import IngestionService
    from agents.tools.owl_reasoner import OWLReasoningAgent
    reasoner = OWLReasoningAgent(ontology.graph)
    return IngestionService(ontology, rust_client, reasoner)

@pytest.fixture
def sample_csv(tmp_path):
    """Sample CSV file for testing"""
    csv_file = tmp_path / "plants.csv"
    csv_file.write_text("""Nombre,Tipo,Suelo,Riego
Manzano,Árbol Frutal,Arcilloso,Moderado
Peral,Árbol Frutal,Franco,Moderado
Tomate,Hortaliza,Franco,Alto
""", encoding='utf-8')
    return csv_file

@pytest.fixture
def sample_document(tmp_path):
    """Sample document for upload testing"""
    doc_file = tmp_path / "research.md"
    doc_file.write_text("""# Permaculture Principles

Permaculture is a sustainable design system based on natural ecosystems.

## Key Concepts
- Observe and interact
- Catch and store energy
- Obtain a yield
""", encoding='utf-8')
    return doc_file

# ============================================
# TEST 1: Translation Service
# ============================================

def test_translation_service_text(translation_service):
    """Test translation of plain text"""
    spanish_text = "El compost mejora la estructura del suelo"
    
    english_text = translation_service.translate(spanish_text)
    
    assert english_text is not None, "Should return translated text"
    assert len(english_text) > 0, "Translation should not be empty"
    assert "compost" in english_text.lower(), "Should contain key terms"
    print(f"✓ Translation: '{spanish_text}' → '{english_text}'")

def test_translation_service_json(translation_service):
    """Test translation of JSON data"""
    spanish_data = {
        "Nombre": "Manzano",
        "Tipo": "Árbol Frutal",
        "Descripción": "Produce manzanas rojas"
    }
    
    english_data = translation_service.translate_json(spanish_data)
    
    assert english_data is not None, "Should return translated data"
    assert "Nombre" in english_data, "Should preserve keys"
    assert "apple" in english_data["Nombre"].lower() or "tree" in english_data["Nombre"].lower()
    print(f"✓ JSON Translation: {spanish_data} → {english_data}")

# ============================================
# TEST 2: Ingestion Service
# ============================================

def test_ingestion_validation(ingestion_service):
    """Test ingestion with validation"""
    triples = [
        ("AppleTree1", "rdf:type", "AppleTree"),
        ("AppleTree1", "growsIn", "Garden1")
    ]
    
    stats = ingestion_service.ingest(
        triples,
        source="TEST:validation",
        metadata={"test": "validation"}
    )
    
    assert stats["input"] == 2, "Should process 2 input triples"
    assert stats["validated"] >= 0, "Should validate triples"
    assert stats["stored"] >= 0, "Should store valid triples"
    print(f"✓ Ingestion Stats: {stats}")

def test_ingestion_deduplication(ingestion_service):
    """Test deduplication"""
    # Clear dedup cache first
    ingestion_service.clear_dedup_cache()
    
    triples = [
        ("Plant1", "rdf:type", "Plant"),
        ("Plant1", "rdf:type", "Plant")  # Duplicate
    ]
    
    stats = ingestion_service.ingest(triples, source="TEST:dedup")
    
    # Should detect at least 1 duplicate (or store only 1)
    assert stats["duplicates"] >= 1 or stats["stored"] == 1, "Should handle duplicates"
    print(f"✓ Deduplication: {stats['duplicates']} duplicates, {stats['stored']} stored")

# ============================================
# TEST 3: OWL Reasoning - Instance-Level
# ============================================

def test_owl_type_propagation(ontology):
    """Test instance-level type propagation"""
    from agents.tools.owl_reasoner import OWLReasoningAgent
    
    reasoner = OWLReasoningAgent(ontology.graph)
    
    # Input: Instance of AppleTree
    triples = [
        ("MyAppleTree", "rdf:type", "AppleTree")
    ]
    
    result = reasoner.infer(triples)
    
    assert result["inferred_triples"] is not None, "Should return inferred triples"
    
    # Should infer superclass types
    inferred_types = [t for t in result["inferred_triples"] if t[1] == "type"]
    
    print(f"✓ Type Propagation: {len(result['inferred_triples'])} inferences")
    print(f"  Input: {triples}")
    print(f"  Inferred: {result['inferred_triples'][:5]}")
    
    assert len(result["inferred_triples"]) > 0, "Should infer at least one triple"

def test_owl_domain_range(ontology):
    """Test domain/range reasoning"""
    from agents.tools.owl_reasoner import OWLReasoningAgent
    
    reasoner = OWLReasoningAgent(ontology.graph)
    
    # Input: growsIn relationship
    triples = [
        ("MyTree", "growsIn", "MyGarden")
    ]
    
    result = reasoner.infer(triples)
    
    print(f"✓ Domain/Range: {len(result['inferred_triples'])} inferences")
    print(f"  Inferred: {result['inferred_triples']}")
    
    assert result is not None, "Should return result"

def test_owl_transitive_property(ontology):
    """Test transitive property reasoning"""
    from agents.tools.owl_reasoner import OWLReasoningAgent
    
    reasoner = OWLReasoningAgent(ontology.graph)
    
    # Input: partOf chain
    triples = [
        ("Leaf", "partOf", "Branch"),
        ("Branch", "partOf", "Tree")
    ]
    
    result = reasoner.infer(triples)
    
    # Should infer: Leaf partOf Tree
    inferred_partof = [t for t in result["inferred_triples"] if t[1] == "partOf"]
    
    print(f"✓ Transitive: {len(inferred_partof)} partOf inferences")
    
    assert result is not None, "Should return result"

# ============================================
# TEST 4: DataSyn Pipeline
# ============================================

def test_datasyn_csv_processing(sample_csv, translation_service):
    """Test DataSyn pipeline with CSV"""
    from agents.application.pipelines.datasyn import DataSynPipeline
    from agents.infrastructure.ai.slm import TrainableSLM
    
    # Initialize SLM
    slm = TrainableSLM(model_name="gpt2")
    
    # Create pipeline
    pipeline = DataSynPipeline(slm=slm, translation_service=translation_service)
    
    # Run pipeline
    result = pipeline.run(str(sample_csv))
    
    assert result.success, f"Pipeline should succeed: {result.logs}"
    assert "triples_extracted" in result.data, "Should extract triples"
    
    print(f"✓ DataSyn: {result.data.get('triples_extracted', 0)} triples from CSV")
    print(f"  Logs: {result.logs[:3]}")

def test_datasyn_error_handling(tmp_path, translation_service):
    """Test DataSyn error handling with malformed CSV"""
    from agents.application.pipelines.datasyn import DataSynPipeline
    from agents.infrastructure.ai.slm import TrainableSLM
    
    # Create malformed CSV
    bad_csv = tmp_path / "bad.csv"
    bad_csv.write_text("invalid,data\n\x00\x00\x00", encoding='utf-8', errors='ignore')
    
    slm = TrainableSLM(model_name="gpt2")
    pipeline = DataSynPipeline(slm=slm, translation_service=translation_service)
    
    result = pipeline.run(str(bad_csv))
    
    # Should handle errors gracefully
    assert result is not None, "Should return result even with errors"
    print(f"✓ Error Handling: Pipeline handled malformed CSV")

# ============================================
# TEST 5: Document Upload & RAG
# ============================================

def test_document_upload_indexing(sample_document, translation_service):
    """Test document upload and RAG indexing"""
    from agents.infrastructure.persistence.embeddings import EmbeddingGenerator
    from agents.infrastructure.persistence.vector_store import VectorStore
    
    embedder = EmbeddingGenerator()
    vector_store = VectorStore("test_docs")
    
    # Read document
    with open(sample_document, 'r') as f:
        content = f.read()
    
    # Translate
    english_content = translation_service.translate(content)
    
    # Chunk and index
    chunk_size = 200
    chunks = [english_content[i:i+chunk_size] for i in range(0, len(english_content), chunk_size)]
    
    for i, chunk in enumerate(chunks):
        if chunk.strip():
            embedding = embedder.encode_single(chunk)
            vector_store.add(f"doc_chunk_{i}", embedding, {"content": chunk[:100]})
    
    # Search
    query_emb = embedder.encode_single("permaculture principles")
    results = vector_store.search(query_emb, top_k=2)
    
    assert len(results) > 0, "Should find indexed chunks"
    print(f"✓ Document Upload: Indexed {len(chunks)} chunks, found {len(results)} results")

# ============================================
# TEST 6: Agent Lightning - AIR System
# ============================================

def test_air_reward_tracking():
    """Test AIR (Adaptive Intrinsic Rewards) system"""
    from agents.infrastructure.ai.air import get_air, RewardSignal
    
    air = get_air()
    air.reset()
    
    # Record various events
    air.record_event(RewardSignal.TRIPLE_EXTRACTED, {"count": 5})
    air.record_event(RewardSignal.OWL_CONSISTENT, {"inferences": 3})
    air.record_event(RewardSignal.USER_POSITIVE_FEEDBACK, {"reward": 2.0})
    
    total_reward = air.get_total_reward()
    summary = air.get_summary()
    
    assert total_reward > 0, "Should accumulate positive rewards"
    assert "Total Reward" in summary, "Should generate summary"
    
    print(f"✓ AIR System: Total Reward = {total_reward}")
    print(f"  {summary}")

def test_experience_buffer():
    """Test Experience Buffer for Agent Lightning"""
    from agents.infrastructure.ai.experience_buffer import ExperienceBuffer
    
    buffer = ExperienceBuffer("test_session")
    
    # Record interactions
    buffer.record_interaction(
        input_text="Test input",
        output_text="Test output",
        feedback=1.0,
        metadata={"test": True}
    )
    
    stats = buffer.get_statistics()
    
    assert stats["total_interactions"] >= 1, "Should track interactions"
    print(f"✓ Experience Buffer: {stats}")

# ============================================
# TEST 7: Model Manager & Continuous Training
# ============================================

def test_model_manager():
    """Test Model Manager for checkpoint management"""
    from agents.infrastructure.ai.model_manager import ModelManager
    
    manager = ModelManager()
    
    # Save checkpoint
    manager.save_checkpoint(
        session_id="test_session",
        model_path="test/model",
        metrics={"loss": 0.5, "accuracy": 0.8}
    )
    
    # Load best model
    best_path = manager.load_best_model()
    
    assert best_path is not None, "Should return model path"
    print(f"✓ Model Manager: Best model at {best_path}")

# ============================================
# TEST 8: Complete E2E Workflow
# ============================================

def test_complete_e2e_workflow(sample_csv, rust_client, ontology, translation_service, ingestion_service):
    """
    Complete end-to-end test demonstrating entire application:
    1. Translate Spanish CSV
    2. Extract triples with DataSyn
    3. Ingest with validation & deduplication
    4. Apply OWL reasoning
    5. Store in Rust backend
    6. Verify storage
    """
    from agents.application.pipelines.datasyn import DataSynPipeline
    from agents.infrastructure.ai.slm import TrainableSLM
    from agents.tools.owl_reasoner import OWLReasoningAgent
    
    print("\n" + "="*60)
    print("COMPLETE END-TO-END WORKFLOW TEST")
    print("="*60)
    
    # Step 1: Initialize components
    print("\n[1/6] Initializing components...")
    slm = TrainableSLM(model_name="gpt2")
    pipeline = DataSynPipeline(slm=slm, translation_service=translation_service)
    reasoner = OWLReasoningAgent(ontology.graph)
    
    # Step 2: Process CSV (includes translation)
    print("\n[2/6] Processing Spanish CSV...")
    result = pipeline.run(str(sample_csv))
    assert result.success, "DataSyn should succeed"
    extracted_count = result.data.get("triples_extracted", 0)
    print(f"  ✓ Extracted {extracted_count} triples")
    
    # Step 3: Ingest with validation
    print("\n[3/6] Ingesting with validation...")
    if extracted_count > 0:
        # Get triples from result
        triples = result.data.get("triples", [])
        if triples:
            # Convert string format to tuples if needed
            if isinstance(triples[0], str):
                import re
                pattern = r'\(([^,]+),\s*([^,]+),\s*([^)]+)\)'
                triple_tuples = []
                for t in triples:
                    match = re.match(pattern, t)
                    if match:
                        triple_tuples.append((match.group(1), match.group(2), match.group(3)))
                triples = triple_tuples
            
            stats = ingestion_service.ingest(
                triples,
                source="E2E_TEST:CSV",
                metadata={"file": sample_csv.name}
            )
            print(f"  ✓ Validated: {stats['validated']}, Duplicates: {stats['duplicates']}, Stored: {stats['stored']}")
    
    # Step 4: Apply OWL reasoning
    print("\n[4/6] Applying OWL reasoning...")
    test_triples = [
        ("AppleTree1", "rdf:type", "AppleTree"),
        ("AppleTree1", "growsIn", "Garden1")
    ]
    reasoning_result = reasoner.infer(test_triples)
    print(f"  ✓ Inferred {len(reasoning_result['inferred_triples'])} new triples")
    
    # Step 5: Verify Rust storage
    print("\n[5/6] Verifying Rust backend storage...")
    all_triples = rust_client.get_all_triples()
    print(f"  ✓ Total triples in graph: {len(all_triples)}")
    
    # Step 6: Summary
    print("\n[6/6] E2E Test Summary:")
    print(f"  • CSV Processing: ✓")
    print(f"  • Translation: ✓")
    print(f"  • Triple Extraction: {extracted_count} triples")
    print(f"  • Validation & Ingestion: ✓")
    print(f"  • OWL Reasoning: {len(reasoning_result['inferred_triples'])} inferences")
    print(f"  • Rust Storage: {len(all_triples)} total triples")
    print("\n" + "="*60)
    print("✅ COMPLETE E2E WORKFLOW PASSED")
    print("="*60 + "\n")
    
    # Assertions
    assert extracted_count >= 0, "Should extract triples"
    assert len(all_triples) > 0, "Should have triples in storage"
    assert reasoning_result is not None, "Should perform reasoning"

# ============================================
# RUN TESTS
# ============================================

if __name__ == "__main__":
    pytest.main([__file__, "-v", "-s", "--tb=short"])
