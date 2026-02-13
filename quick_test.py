#!/usr/bin/env python3
import subprocess
import time
import grpc
import json
import sys
import os

# Add path to generated stubs
sys.path.append(os.path.join(os.path.dirname(__file__), 'scripts/generated'))
try:
    import semantic_engine_pb2 as pb2
    import semantic_engine_pb2_grpc as pb2_grpc
except ImportError:
    print("Error: Could not import generated stubs.")
    sys.exit(1)

def start_synapse():
    """Start synapse server with auth token"""
    env = os.environ.copy()
    env['SYNAPSE_AUTH_TOKENS'] = '{"test": ["*"]}'
    # Kill any existing synapse
    subprocess.run(['pkill', '-f', 'synapse'], capture_output=True)
    time.sleep(1)
    # Start new synapse
    synapse_path = os.path.join(os.path.dirname(__file__), 'bin/synapse')
    proc = subprocess.Popen([synapse_path], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    time.sleep(2)  # Wait for server to start
    # Check if process is still running
    if proc.poll() is not None:
        stdout, stderr = proc.communicate()
        print("Synapse failed to start:")
        print(stderr.decode())
        return None
    return proc

def query_with_token(query, namespace="default", token="test"):
    """Execute SPARQL query with bearer token"""
    channel = grpc.insecure_channel('localhost:50051')
    stub = pb2_grpc.SemanticEngineStub(channel)
    metadata = [('authorization', f'Bearer {token}')]
    req = pb2.SparqlRequest(query=query, namespace=namespace)
    try:
        res = stub.QuerySparql(req, metadata=metadata)
        return json.loads(res.results_json)
    except grpc.RpcError as e:
        print(f"gRPC Error: {e.details()}")
        return None

def main():
    print("Starting synapse server...")
    proc = start_synapse()
    if proc is None:
        print("Failed to start synapse.")
        return
    print("Server started. Querying...")
    result = query_with_token("SELECT ?s ?p ?o WHERE { ?s ?p ?o } LIMIT 5")
    if result:
        print("Success! Results:")
        print(json.dumps(result, indent=2))
    else:
        print("Query failed.")
    # Cleanup
    proc.terminate()
    proc.wait()
    print("Done.")

if __name__ == "__main__":
    main()