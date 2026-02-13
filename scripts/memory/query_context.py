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

def query_sparql(query, namespace="conversation", token="test"):
    channel = grpc.insecure_channel('localhost:50051')
    stub = pb2_grpc.SemanticEngineStub(channel)
    metadata = [('authorization', f'Bearer {token}')]
    req = pb2.SparqlRequest(query=query, namespace=namespace)
    try:
        res = stub.QuerySparql(req, metadata=metadata)
        return json.loads(res.results_json)
    except grpc.RpcError as e:
        print(f"Query error: {e.details()}")
        return None

if __name__ == "__main__":
    # Query what Pelayo has asked
    results = query_sparql("""
        SELECT ?predicate ?object WHERE {
            <http://synapse.os/Pelayo> ?predicate ?object .
        }
    """)
    if results:
        print("📚 Contexto recuperado de Synapse:")
        for row in results:
            pred = row.get('?predicate', '').replace('http://synapse.os/', '')
            obj = row.get('?object', '').replace('http://synapse.os/', '')
            print(f"  - Pelayo {pred} {obj}")
    else:
        print("No hay contexto previo.")