"""
Dependency Injection Container
Manages singleton instances of core services.
"""
from typing import Optional, Any
import os

class DIContainer:
    _instance = None
    
    def __init__(self):
        self._services = {}
    
    @classmethod
    def get_instance(cls):
        if cls._instance is None:
            cls._instance = DIContainer()
        return cls._instance

    def graph_repository(self):
        # Return Rust client wrapper
        from agents.infrastructure.web.client import get_client
        return get_client()

    def embedding_service(self):
        if "embedding_service" not in self._services:
            embedding_type = os.getenv("EMBEDDING_TYPE", "standard")

            if embedding_type == "kv":
                print("Initializing KV-Embedding Generator...")
                from agents.infrastructure.persistence.kv_embeddings import KVEmbeddingGenerator
                # Allow configuring model via env var, default to SmolLM2 for efficiency or Mistral for quality
                model_name = os.getenv("KV_EMBEDDING_MODEL", "HuggingFaceTB/SmolLM2-135M")
                self._services["embedding_service"] = KVEmbeddingGenerator(model_name=model_name)
            else:
                print("Initializing Standard Embedding Generator...")
                from agents.infrastructure.persistence.embeddings import EmbeddingGenerator
                self._services["embedding_service"] = EmbeddingGenerator()

        return self._services["embedding_service"]

    def vector_store(self, collection_name: str):
        import os
        from agents.infrastructure.persistence.vector_store import VectorStore
        from qdrant_client import QdrantClient
        
        if "qdrant_client" not in self._services:
             qdrant_url = os.getenv("QDRANT_URL")
             if qdrant_url:
                 # Connect to Qdrant server
                 self._services["qdrant_client"] = QdrantClient(url=qdrant_url)
             else:
                 # Fallback to local storage
                 self._services["qdrant_client"] = QdrantClient(path="./qdrant_storage")

        # Determine dimension based on embedding service
        # We need to instantiate embedding service to know the dimension if we want to be dynamic,
        # OR we rely on configuration.
        # But vector_store usually doesn't call embedding_service, the caller does.
        # However, vector_store needs to know the dimension for initialization.

        # We can peek at the embedding service if it's already initialized, or check env var.
        embedding_type = os.getenv("EMBEDDING_TYPE", "standard")
        dimension = 384 # Default for standard

        if embedding_type == "kv":
            # If KV, we need to know the model hidden size.
            # This is tricky without loading the model.
            # We can try to get it from the service if initialized.
            if "embedding_service" in self._services:
                service = self._services["embedding_service"]
                if hasattr(service, "hidden_size"):
                    dimension = service.hidden_size
            else:
                # If not initialized, we might default or force initialization?
                # Forcing initialization is safer to ensure consistency.
                service = self.embedding_service()
                if hasattr(service, "hidden_size"):
                    dimension = service.hidden_size
                else:
                    # Fallback if somehow not available (e.g. standard generator doesn't have it explicitly maybe?)
                    # Standard generator (embeddings.py) uses all-MiniLM-L6-v2 which is 384.
                    dimension = 384

        return VectorStore(
            collection_name=collection_name, 
            dimension=dimension,
            client=self._services["qdrant_client"]
        )

    def ontology_service(self):
        from agents.domain.services.ontology import OntologyService
        if "ontology_service" not in self._services:
            # Load default ontologies
            self._services["ontology_service"] = OntologyService(["ontology/core.owl", "ontology/frontend.owl"])
        return self._services["ontology_service"]

    def reasoning_engine(self):
        from agents.tools.owl_reasoner import OWLReasoningAgent
        ontology = self.ontology_service()
        return OWLReasoningAgent(ontology.graph)

    def translation_service(self):
        if "translation_service" not in self._services:
            from agents.domain.services.translation_service import TranslationService
            self._services["translation_service"] = TranslationService()
        return self._services["translation_service"]

    def slm(self):
        # Local ML training/inference disabled as per user request (no GPU environment)
        return None

def get_container():
    return DIContainer.get_instance()
