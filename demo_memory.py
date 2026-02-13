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
        return response
    except grpc.RpcError as e:
        print(f"Ingest error: {e.details()}")
        return None

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
    print("🧠 Demostración de Memoria Continua con Synapse")
    print("="*50)
    
    # 1. Consultar contexto previo
    print("1. Consultando contexto previo...")
    results = query_sparql("""
        SELECT ?predicate ?object WHERE {
            <http://synapse.os/Pelayo> ?predicate ?object .
        }
    """)
    if results:
        print("   Contexto recuperado:")
        for row in results:
            pred = row.get('?predicate', '').replace('http://synapse.os/', '')
            obj = row.get('?object', '').replace('http://synapse.os/', '')
            print(f"   - Pelayo {pred} {obj}")
    else:
        print("   Sin contexto previo.")
    
    # 2. Ingestar nueva interacción
    print("\n2. Ingresando nueva interacción...")
    resp = ingest_triple("Robin", "consulted", "previous context from Synapse")
    if resp:
        print(f"   Triples añadidos: {resp.edges_added}")
    
    # 3. Mostrar todo el contenido
    print("\n3. Estado actual del grafo de conversación:")
    all_triples = query_sparql("SELECT ?s ?p ?o WHERE { ?s ?p ?o }")
    if all_triples:
        for row in all_triples:
            s = row.get('?s', '').replace('http://synapse.os/', '')
            p = row.get('?p', '').replace('http://synapse.os/', '')
            o = row.get('?o', '').replace('http://synapse.os/', '')
            print(f"   - {s} {p} {o}")
    print("\n✅ Demostración completada.")