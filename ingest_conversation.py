#!/usr/bin/env python3
import grpc
import json
import sys
import os

sys.path.append(os.path.join(os.path.dirname(__file__), 'scripts/generated'))
try:
    import semantic_engine_pb2 as pb2
    import semantic_engine_pb2_grpc as pb2_grpc
except ImportError:
    print("Error: Could not import generated stubs.")
    sys.exit(1)

def ingest_triple(subject, predicate, object, namespace="conversation", token="test"):
    channel = grpc.insecure_channel('localhost:50051')
    stub = pb2_grpc.SemanticEngineStub(channel)
    metadata = [('authorization', f'Bearer {token}')]
    triple = pb2.Triple(subject=subject, predicate=predicate, object=object)
    request = pb2.IngestRequest(triples=[triple], namespace=namespace)
    try:
        response = stub.IngestTriples(request, metadata=metadata)
        print(f"Ingested: {subject} {predicate} {object}")
        return response
    except grpc.RpcError as e:
        print(f"Ingest error: {e.details()}")
        return None

def query_all(namespace="conversation", token="test"):
    channel = grpc.insecure_channel('localhost:50051')
    stub = pb2_grpc.SemanticEngineStub(channel)
    metadata = [('authorization', f'Bearer {token}')]
    req = pb2.SparqlRequest(query="SELECT ?s ?p ?o WHERE { ?s ?p ?o }", namespace=namespace)
    try:
        res = stub.QuerySparql(req, metadata=metadata)
        return json.loads(res.results_json)
    except grpc.RpcError as e:
        print(f"Query error: {e.details()}")
        return None

if __name__ == "__main__":
    # Ingest current conversation
    ingest_triple("Pelayo", "asked", "about synapse MCP integration")
    ingest_triple("Robin", "investigated", "mcporter configuration")
    ingest_triple("Robin", "found", "synapse gRPC server operational")
    ingest_triple("Robin", "proposed", "SDK Python integration as alternative")
    # Query and print
    results = query_all()
    if results:
        print("Current triples in 'conversation' namespace:")
        print(json.dumps(results, indent=2))
    else:
        print("No triples found.")