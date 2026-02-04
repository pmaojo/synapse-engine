import unittest
import os
from agents.infrastructure.di_container import DIContainer
from agents.infrastructure.persistence.kv_embeddings import KVEmbeddingGenerator
from agents.infrastructure.persistence.embeddings import EmbeddingGenerator

class TestDIContainerIntegration(unittest.TestCase):
    def setUp(self):
        # Reset singleton instance
        DIContainer._instance = None

    def test_standard_embedding_service(self):
        os.environ["EMBEDDING_TYPE"] = "standard"
        container = DIContainer.get_instance()
        service = container.embedding_service()
        self.assertIsInstance(service, EmbeddingGenerator)

        vs = container.vector_store("test_standard")
        self.assertEqual(vs.dimension, 384)

    def test_kv_embedding_service(self):
        os.environ["EMBEDDING_TYPE"] = "kv"
        # Use small model for test speed
        os.environ["KV_EMBEDDING_MODEL"] = "HuggingFaceTB/SmolLM2-135M"

        container = DIContainer.get_instance()
        service = container.embedding_service()
        self.assertIsInstance(service, KVEmbeddingGenerator)

        # Verify dimension propagation
        # SmolLM2-135M has hidden size 576
        vs = container.vector_store("test_kv")
        self.assertEqual(vs.dimension, 576)

if __name__ == "__main__":
    unittest.main()
