import sys
import os

# Add python-sdk to path
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "../python-sdk")))

from synapse.infrastructure.web.client import SemanticEngineClient
import synapse.infrastructure.web.semantic_engine_pb2 as pb2

class ExtendedClient(SemanticEngineClient):
    """
    Extended SemanticEngineClient with methods for all gRPC endpoints.
    """

    def apply_reasoning(self, namespace: str = "default", strategy: str = "rdfs", materialize: bool = False):
        if not self.connected:
            if not self.connect(): return {"error": "Not connected"}

        strategy_enum = pb2.ReasoningStrategy.RDFS
        if strategy.lower() in ["owlrl", "owl-rl"]:
            strategy_enum = pb2.ReasoningStrategy.OWLRL

        try:
            request = pb2.ReasoningRequest(
                namespace=namespace,
                strategy=strategy_enum,
                materialize=materialize
            )
            response = self.stub.ApplyReasoning(request, metadata=self._get_metadata())
            return {
                "success": response.success,
                "triples_inferred": response.triples_inferred,
                "message": response.message
            }
        except Exception as e:
            return {"error": str(e)}

    def query_sparql(self, query: str, namespace: str = "default"):
        if not self.connected:
            if not self.connect(): return {"error": "Not connected"}

        try:
            request = pb2.SparqlRequest(
                query=query,
                namespace=namespace
            )
            response = self.stub.QuerySparql(request, metadata=self._get_metadata())
            return response.results_json
        except Exception as e:
            return {"error": str(e)}

    def get_neighbors_full(self, node_id: int, namespace: str = "", direction: str = "outgoing"):
        if not self.connected:
            if not self.connect(): return []

        try:
            request = pb2.NodeRequest(
                node_id=node_id,
                namespace=namespace,
                direction=direction
            )
            response = self.stub.GetNeighbors(request, metadata=self._get_metadata())
            return [
                {
                    "node_id": n.node_id,
                    "edge_type": n.edge_type,
                    "uri": n.uri,
                    "direction": n.direction,
                    "score": n.score
                }
                for n in response.neighbors
            ]
        except Exception as e:
            print(f"Error: {e}")
            return []

    def get_neighbors_by_uri(self, uri: str, namespace: str = "default", direction: str = "outgoing"):
        # Helper to resolve ID first
        node_id = self.resolve_id(uri, namespace)
        if node_id is None:
            print(f"Node not found: {uri}")
            return []

        return self.get_neighbors_full(node_id, namespace, direction)
