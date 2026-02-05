"""
gRPC Client for Semantic Engine (Rust Backend)
Provides a Python interface to the production Rust graph storage
"""
import grpc
from typing import List, Tuple, Optional
import sys
import os

try:
    # Import generated protobuf files with absolute path
    import synapse.infrastructure.web.semantic_engine_pb2 as pb2
    import synapse.infrastructure.web.semantic_engine_pb2_grpc as pb2_grpc
except ImportError as e:
    print(f"⚠️  gRPC stubs not found: {e}")
    print("Run: python -m grpc_tools.protoc -I./crates/semantic-engine/proto --python_out=./agents/infrastructure/web --grpc_python_out=./agents/infrastructure/web ./crates/semantic-engine/proto/semantic_engine.proto")
    pb2 = None
    pb2_grpc = None

class SemanticEngineClient:
    """
    Python client for the Rust semantic-engine gRPC server.
    Provides persistent, production-grade graph storage.
    """
    def __init__(self, host: str = "localhost", port: int = 50051):
        self.address = f"{host}:{port}"
        self.channel = None
        self.stub = None
        self.connected = False
        
    def connect(self) -> bool:
        """Establish connection to Rust server"""
        try:
            self.channel = grpc.insecure_channel(self.address)
            self.stub = pb2_grpc.SemanticEngineStub(self.channel)
            # Test connection
            grpc.channel_ready_future(self.channel).result(timeout=2)
            self.connected = True
            print(f"✅ Connected to Rust backend at {self.address}")
            return True
        except Exception as e:
            self.connected = False
            print(f"⚠️  Could not connect to Rust backend: {e}")
            return False
    
    def ingest_triples(self, triples: List[dict], namespace: str = "") -> dict:
        """
        Send triples to Rust backend for storage.
        
        Args:
            triples: List of dicts with 'subject', 'predicate', 'object', and optional 'provenance'
            namespace: Optional tenant ID for multi-tenancy
            
        Returns:
            dict with 'nodes_added' and 'edges_added'
        """
        if not self.connected:
            if not self.connect():
                return {"error": "Not connected to Rust backend"}
        
        try:
            pb_triples = []
            for t in triples:
                triple_msg = pb2.Triple(
                    subject=t["subject"],
                    predicate=t["predicate"],
                    object=t["object"]
                )
                if "provenance" in t and t["provenance"]:
                    prov = t["provenance"]
                    triple_msg.provenance.CopyFrom(pb2.Provenance(
                        source=prov.get("source", ""),
                        timestamp=prov.get("timestamp", ""),
                        method=prov.get("method", "")
                    ))
                pb_triples.append(triple_msg)

            request = pb2.IngestRequest(
                triples=pb_triples,
                namespace=namespace
            )
            response = self.stub.IngestTriples(request)
            return {
                "nodes_added": response.nodes_added,
                "edges_added": response.edges_added
            }
        except Exception as e:
            return {"error": str(e)}
    
    def get_neighbors(self, node_id: int, namespace: str = "") -> List[dict]:
        """Get neighbors of a node by ID"""
        if not self.connected:
            if not self.connect():
                return []
        
        try:
            request = pb2.NodeRequest(node_id=node_id, namespace=namespace)
            response = self.stub.GetNeighbors(request)
            return [
                {"node_id": n.node_id, "edge_type": n.edge_type}
                for n in response.neighbors
            ]
        except Exception as e:
            print(f"Error getting neighbors: {e}")
            return []
    
    def resolve_id(self, name: str, namespace: str = "") -> Optional[int]:
        """Resolve a string name to a node ID"""
        if not self.connected:
            if not self.connect():
                return None
        
        try:
            request = pb2.ResolveRequest(content=name, namespace=namespace)
            response = self.stub.ResolveId(request)
            return response.node_id if response.found else None
        except Exception as e:
            print(f"Error resolving ID: {e}")
            return None
    
    def get_all_triples(self, namespace: str = "") -> List[dict]:
        """Get all stored triples from Rust backend, including provenance"""
        if not self.connected:
            if not self.connect():
                return []
        
        try:
            request = pb2.EmptyRequest(namespace=namespace)
            response = self.stub.GetAllTriples(request)
            result = []
            for t in response.triples:
                triple_dict = {
                    "subject": t.subject,
                    "predicate": t.predicate,
                    "object": t.object
                }
                if t.HasField("provenance"):
                    triple_dict["provenance"] = {
                        "source": t.provenance.source,
                        "timestamp": t.provenance.timestamp,
                        "method": t.provenance.method
                    }
                result.append(triple_dict)
            return result
        except Exception as e:
            print(f"Error getting triples: {e}")
            return []

    def delete_tenant_data(self, namespace: str) -> dict:
        """Delete all data for a tenant"""
        if not self.connected:
            if not self.connect():
                return {"success": False, "message": "Not connected to Rust backend"}

        try:
            request = pb2.EmptyRequest(namespace=namespace)
            response = self.stub.DeleteTenantData(request)
            return {
                "success": response.success,
                "message": response.message
            }
        except Exception as e:
            return {"success": False, "message": str(e)}
    
    def close(self):
        """Close the gRPC channel"""
        if self.channel:
            self.channel.close()
            self.connected = False

# Global singleton instance
_client = None

def get_client() -> SemanticEngineClient:
    """Get or create the global client instance"""
    global _client
    if _client is None:
        _client = SemanticEngineClient()
        _client.connect()
    return _client
