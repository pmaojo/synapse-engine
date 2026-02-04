"""Vector store interface using Qdrant"""
from typing import List, Dict, Any, Optional
import numpy as np
from dataclasses import dataclass
import os
from qdrant_client import QdrantClient
from qdrant_client.http import models

@dataclass
class VectorSearchResult:
    node_id: str
    score: float
    metadata: Dict[str, Any]

class VectorStore:
    """Vector store implementation using Qdrant"""
    
    def __init__(self, collection_name: str = "semantic_graph", dimension: int = 384, url: str = None, client: Optional[QdrantClient] = None, tenant_id: str = None):
        self.base_collection_name = collection_name
        self.dimension = dimension
        self.tenant_id = tenant_id # Default tenant ID, can be overridden in methods
        
        # Use injected client, or create new one
        if client:
            self.client = client
        elif url:
            self.client = QdrantClient(url=url)
        else:
            # Check env var or fallback to local persistence
            env_url = os.getenv("QDRANT_URL")
            if env_url:
                self.client = QdrantClient(url=env_url)
            else:
                self.client = QdrantClient(path="./qdrant_storage")
            
        # Ensure default collection exists if tenant_id is provided or implicit default
        self._ensure_collection(self.get_collection_name())

    def get_collection_name(self, tenant_id: Optional[str] = None) -> str:
        """Get the collection name, optionally suffixed with tenant_id"""
        # If explicit tenant_id provided, use it
        if tenant_id:
            return f"{self.base_collection_name}_{tenant_id}"
        # If default tenant_id exists, use it
        if self.tenant_id:
            return f"{self.base_collection_name}_{self.tenant_id}"
        # Fallback to base name (shared collection or single tenant)
        return self.base_collection_name
        
    def _ensure_collection(self, collection_name: str):
        """Ensure the collection exists"""
        collections = self.client.get_collections().collections
        exists = any(c.name == collection_name for c in collections)
        
        if not exists:
            self.client.create_collection(
                collection_name=collection_name,
                vectors_config=models.VectorParams(
                    size=self.dimension,
                    distance=models.Distance.COSINE
                )
            )
        
    def add(self, node_id: str, vector: np.ndarray, metadata: Optional[Dict] = None, tenant_id: Optional[str] = None):
        """Add a vector to the store"""
        if vector.shape[0] != self.dimension:
            raise ValueError(f"Vector dimension mismatch: {vector.shape[0]} != {self.dimension}")
            
        collection = self.get_collection_name(tenant_id)
        # Ensure collection exists (lazy creation for new tenants)
        self._ensure_collection(collection)

        # Qdrant requires integer or UUID IDs usually, but supports string UUIDs.
        # For simplicity, we'll hash the string node_id to a UUID if it's not one,
        # or just use a point ID generation strategy.
        # Here we use a deterministic UUID based on node_id.
        import uuid
        point_id = str(uuid.uuid5(uuid.NAMESPACE_DNS, node_id))
        
        self.client.upsert(
            collection_name=collection,
            points=[
                models.PointStruct(
                    id=point_id,
                    vector=vector.tolist(),
                    payload={
                        "original_id": node_id,
                        **(metadata or {})
                    }
                )
            ]
        )
        
    def search(self, query_vector: np.ndarray, top_k: int = 10, tenant_id: Optional[str] = None) -> List[VectorSearchResult]:
        """Search for similar vectors"""
        from qdrant_client.models import SearchRequest
        
        collection = self.get_collection_name(tenant_id)
        # Check if collection exists before querying to avoid error
        # Actually Qdrant raises error if collection doesn't exist.
        # But for search, maybe we just return empty if it doesn't exist?
        # For performance, we assume it exists or catch error.
        
        try:
            results = self.client.query_points(
                collection_name=collection,
                query=query_vector.tolist(),
                limit=top_k
            )

            return [
                VectorSearchResult(
                    node_id=hit.payload.get("original_id", str(hit.id)),
                    score=hit.score,
                    metadata=hit.payload
                )
                for hit in results.points
            ]
        except Exception as e:
            # If collection doesn't exist, return empty
            if "Not found: Collection" in str(e):
                return []
            raise e
    
    def delete(self, node_id: str, tenant_id: Optional[str] = None):
        """Remove a vector"""
        import uuid
        point_id = str(uuid.uuid5(uuid.NAMESPACE_DNS, node_id))

        collection = self.get_collection_name(tenant_id)

        self.client.delete(
            collection_name=collection,
            points_selector=models.PointIdsList(
                points=[point_id]
            )
        )

    def delete_collection(self, tenant_id: Optional[str] = None):
        """Delete the entire collection for the current tenant"""
        collection = self.get_collection_name(tenant_id)
        try:
            self.client.delete_collection(collection_name=collection)
            # Do NOT recreate immediately, wait for next add
        except Exception as e:
            print(f"Error deleting collection {collection}: {e}")
