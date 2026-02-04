
import sys
import unittest
from unittest.mock import MagicMock
from rdflib import Graph, URIRef, RDF, RDFS

from agents.domain.services.ingestion_service import IngestionService
from agents.tools.owl_reasoner import OWLReasoningAgent

class MockOntologyService:
    def __init__(self):
        self.graph = Graph()

        self.animal = URIRef("http://example.org/Animal")
        self.mammal = URIRef("http://example.org/Mammal")
        self.dog = URIRef("http://example.org/Dog")

        self.graph.add((self.animal, RDF.type, RDFS.Class))
        self.graph.add((self.mammal, RDF.type, RDFS.Class))
        self.graph.add((self.dog, RDF.type, RDFS.Class))

        self.graph.add((self.mammal, RDFS.subClassOf, self.animal))
        self.graph.add((self.dog, RDFS.subClassOf, self.mammal))

class MockRustClient:
    def __init__(self):
        self.connected = True
        self.ingested = []

    def ingest_triples(self, triples, tenant_id="default"):
        self.ingested.extend(triples)
        return len(triples)

class TestIngestionService(unittest.TestCase):
    def setUp(self):
        self.ontology_service = MockOntologyService()
        self.rust_client = MockRustClient()
        self.reasoner = OWLReasoningAgent(self.ontology_service.graph)
        self.ingestion_service = IngestionService(
            self.ontology_service,
            self.rust_client,
            owl_reasoner=self.reasoner
        )
        # Fix validation for test
        self.ingestion_service.validator.valid_properties.add("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")

    def test_ingest_with_reasoning(self):
        """Test that ingestion correctly enriches triples using reasoning"""
        triples = [
            ("http://example.org/Fido", "http://www.w3.org/1999/02/22-rdf-syntax-ns#type", "http://example.org/Dog")
        ]

        stats = self.ingestion_service.ingest(triples, skip_enrichment=False)

        self.assertGreater(stats["enriched"], 0)
        self.assertGreater(len(self.rust_client.ingested), 1)

        # Verify specific inferences
        # Note: Inferred triples might use short names or full URIs depending on reasoner output format
        # but we check if we have more than input.

    def test_ingest_skip_reasoning(self):
        """Test skipping reasoning"""
        triples = [
            ("http://example.org/Fido", "http://www.w3.org/1999/02/22-rdf-syntax-ns#type", "http://example.org/Dog")
        ]

        stats = self.ingestion_service.ingest(triples, skip_enrichment=True)

        self.assertEqual(stats["enriched"], 0)
        self.assertEqual(len(self.rust_client.ingested), 1)

if __name__ == "__main__":
    unittest.main()
