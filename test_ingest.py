#!/usr/bin/env python3
import subprocess
import time
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

def start_synapse():
    env = os.environ.copy()
    env['SYNAPSE_AUTH_TOKENS'] = '{"test": ["*"]}'
    # Kill any existing synapse
    subprocess.run(['pkill', '-f', 'synapse'], capture_output=True)
    time.sleep(1)
    synapse_path = os.path.join(os.path.dirname(__file__), 'bin/synapse')
    proc = subprocess.Popen([synapse_path], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    time.sleep(2)
    if proc.poll() is not None:
        stdout, stderr = proc.communicate()
        print("Synapse failed to start:", stderr.decode())
        return None
    return proc

def ingest_triple(subject, predicate, object, namespace="test", token="test"):
    channel = grpc.insecure_channel('localhost:50051')
    stub = pb2_grpc.SemanticEngineStub(channel)
    metadata = [('authorization', f'Bearer {token}')]
    triple = pb2.Triple(subject=subject, predicate=predicate, object=object)
    request = pb2.IngestRequest(triples=[triple], namespace=namespace)
    try:
        response = stub.IngestTriples(request, metadata=metadata)
        print(f"Ingested: {subject} {predicate} {object}")
        print(f"Nodes added: {response.nodes_added}, Edges added: {response.edges_added}")
        return response
    except grpc.RpcError as e:
        print(f"Ingest error: {e.details()}")
        return None

def query_sparql(query, namespace="test", token="test"):
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

def main():
    print("Starting synapse...")
    proc = start_synapse()
    if proc is None:
        return
    print("Ingesting test triple...")
    ingest_triple("Pelayo", "asked", "about synapse MCP", namespace="conversation")
    print("Querying...")
    result = query_sparql("SELECT ?s ?p ?o WHERE { ?s ?p ?o }", namespace="conversation")
    if result:
        print("Query results:", json.dumps(result, indent=2))
    else:
        print("No results.")
    proc.terminate()
    proc.wait()
    print("Done.")

if __name__ == "__main__":
    main()