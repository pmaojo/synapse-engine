#!/usr/bin/env python3
import grpc
import sys
import os
sys.path.append(os.path.join(os.path.dirname(__file__), 'scripts/generated'))
try:
    import semantic_engine_pb2 as pb2
    import semantic_engine_pb2_grpc as pb2_grpc
except ImportError:
    print("Error: Could not import generated stubs.")
    sys.exit(1)

def ingest(subject, predicate, object, namespace="conversation", token="test"):
    channel = grpc.insecure_channel('localhost:50051')
    stub = pb2_grpc.SemanticEngineStub(channel)
    metadata = [('authorization', f'Bearer {token}')]
    triple = pb2.Triple(subject=subject, predicate=predicate, object=object)
    request = pb2.IngestRequest(triples=[triple], namespace=namespace)
    try:
        stub.IngestTriples(request, metadata=metadata)
        print(f"Ingested: {subject} {predicate} {object}")
    except grpc.RpcError as e:
        print(f"Error: {e.details()}")

if __name__ == "__main__":
    ingest("Pelayo", "is_fixing", "synapse MCP handshake")
    ingest("Robin", "is_ready", "to assist if needed")