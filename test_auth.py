import grpc
import json
import sys
sys.path.append('/home/robin/workspace/skills/synapse/scripts/generated')
try:
    import semantic_engine_pb2 as pb2
    import semantic_engine_pb2_grpc as pb2_grpc
except ImportError:
    print("Error: Could not import generated stubs.")
    sys.exit(1)

def query_with_token(query, namespace="default", token="test"):
    channel = grpc.insecure_channel('localhost:50051')
    if token:
        # Add authorization metadata
        metadata = (('authorization', f'Bearer {token}'),)
        intercept_channel = grpc.intercept_channel(channel, *[grpc.unary_unary_interceptor(lambda req, metadata, invoker, **kwargs: invoker(req, metadata + metadata))])
        stub = pb2_grpc.SemanticEngineStub(intercept_channel)
    else:
        stub = pb2_grpc.SemanticEngineStub(channel)
    
    req = pb2.SparqlRequest(query=query, namespace=namespace)
    try:
        res = stub.QuerySparql(req)
        print(json.dumps(json.loads(res.results_json), indent=2))
    except grpc.RpcError as e:
        print(f"gRPC Error: {e.details()}")
        print(f"Code: {e.code()}")

if __name__ == "__main__":
    # Start synapse server first
    import subprocess, os, time
    # Set auth token environment variable
    os.environ['SYNAPSE_AUTH_TOKENS'] = '{"test": ["*"]}'
    # Kill any existing synapse
    subprocess.run(['pkill', '-f', 'synapse'], capture_output=True)
    # Start synapse
    synapse_path = '/home/robin/workspace/skills/synapse/bin/synapse'
    proc = subprocess.Popen([synapse_path], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    time.sleep(2)  # Wait for server to start
    # Test query
    query_with_token("SELECT ?s ?p ?o WHERE { ?s ?p ?o } LIMIT 5", namespace="default", token="test")
    proc.terminate()
    proc.wait()