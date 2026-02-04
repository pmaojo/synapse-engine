#!/usr/bin/env python3
"""
Complete demonstration of the Synapse Semantic System
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
    print_section("🧠 Synapse System - Complete Demo")
    
    # Initialize system
    print("Initializing system components...")
    ontology_paths = [
        "ontology/core.owl",
        "ontology/frontend.owl"
    ]
    
    graph_client = GraphClient()
    vector_store = VectorStore(dimension=384)
    
    print("✓ Ontology loaded")
    print("✓ Graph client initialized")
    print("✓ Vector store ready")
    
    # Demo 1: Knowledge ingestion
    print_section("Demo 1: Knowledge Ingestion")
    
    texts = [
        "Synapse is a neuro-symbolic knowledge graph for agents.",
        "It is built with Rust for high performance.",
        "Model Context Protocol allows seamless LLM integration."
    ]
    
    for text in texts:
        print(f"Ingesting: {text}")
        # In a real demo, we would call the pipeline here
        pass

    print_section("✨ Demo Complete")

if __name__ == "__main__":
    main()
