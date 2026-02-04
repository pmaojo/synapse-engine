#!/usr/bin/env python3
"""
Complete demonstration of the Semantic System
"""
import sys
import os
import logging

# Setup logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)

sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from agents.infrastructure.persistence.graph_client import GraphClient
from agents.infrastructure.persistence.vector_store import VectorStore

def print_section(title):
    print(f"\n{'='*60}")
    print(f"  {title}")
    print(f"{'='*60}\n")

def main():
    print_section("🌱 Semantic System - Complete Demo")
    
    # Initialize system
    print("Initializing system components...")
    ontology_paths = [
        "ontology/core.owl",
        "ontology/agriculture.owl"
    ]
    
    graph_client = GraphClient()
    vector_store = VectorStore(dimension=384)
    pipeline = SemanticPipeline(ontology_paths, graph_client, vector_store)
    
    print("✓ Ontology loaded")
    print("✓ Graph client initialized")
    print("✓ Vector store ready")
    print("✓ Pipeline configured")
    
    # Demo 1: Process agricultural text
    print_section("Demo 1: Processing Agricultural Text")
    
    texts = [
        "A Food Forest is a Permaculture system that has Swales.",
        "Organic Farming is a sustainable agricultural practice.",
        "Regenerative Agriculture improves soil health and biodiversity."
    ]
    
    for i, text in enumerate(texts, 1):
        print(f"\n[{i}] Input: '{text}'")
        result = pipeline.process_text(text)
        print(f"    → Extracted: {result['extracted']} triples")
        print(f"    → Mapped: {result['mapped']} triples")
        print(f"    → Validated: {result['validated']} triples")
        
        if result['triples']:
            print(f"    → Valid triples:")
            for triple in result['triples']:
                print(f"       • {triple['subject']}")
                print(f"         --{triple['predicate']}-->")
                print(f"         {triple['object']}")
    
    # Demo 2: Query with RAG
    print_section("Demo 2: RAG Query")
    
    query = "What is Permaculture?"
    print(f"Query: '{query}'")
    rag_result = pipeline.query(query, mode="rag")
    print(f"\nResults: {len(rag_result['results'])} items found")
    print(f"Context:\n{rag_result['context']}")
    
    # Demo 3: SPARQL Query
    print_section("Demo 3: SPARQL Query")
    
    print("Getting all OWL classes...")
    classes = pipeline.sparql_engine.get_all_classes()
    print(f"Found {len(classes)} classes:")
    for cls in classes[:10]:  # Show first 10
        print(f"  • {cls.get('class', 'N/A')} - {cls.get('label', 'No label')}")
    
    print("\nGetting all OWL properties...")
    props = pipeline.sparql_engine.get_all_properties()
    print(f"Found {len(props)} properties:")
    for prop in props[:10]:  # Show first 10
        print(f"  • {prop.get('prop', 'N/A')} - {prop.get('label', 'No label')}")
    
    # Demo 4: Vector Search
    print_section("Demo 4: Vector Similarity Search")
    
    print(f"Vector store contains {len(vector_store.vectors)} embeddings")
    if vector_store.vectors:
        query_text = "sustainable farming"
        print(f"\nSearching for: '{query_text}'")
        query_vec = pipeline.embedder.encode_single(query_text)
        results = vector_store.search(query_vec, top_k=3)
        print(f"Top {len(results)} similar nodes:")
        for r in results:
            print(f"  • Node: {r.node_id} (score: {r.score:.3f})")
            print(f"    Metadata: {r.metadata}")
    
    print_section("✨ Demo Complete")
    print("\nSystem Statistics:")
    print(f"  • Ontology classes: {len(pipeline.ontology.classes)}")
    print(f"  • Ontology properties: {len(pipeline.ontology.properties)}")
    print(f"  • Vectors stored: {len(vector_store.vectors)}")
    print(f"  • Graph triples: (stored in Rust engine)")

if __name__ == "__main__":
    main()
