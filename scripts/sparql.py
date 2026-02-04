import sys
import os
import grpc
import argparse
import json

# Setup paths
sys.path.append(os.path.join(os.path.dirname(__file__), 'generated'))
try:
    import semantic_engine_pb2 as pb2
    import semantic_engine_pb2_grpc as pb2_grpc
except ImportError:
    print("Error: Could not import generated stubs. Run protoc generation first.")
    sys.exit(1)

def query_sparql(query, namespace="robin_os"):
    channel = grpc.insecure_channel('localhost:50051')
    stub = pb2_grpc.SemanticEngineStub(channel)
    
    req = pb2.SparqlRequest(
        query=query,
        namespace=namespace
    )
    
    try:
        # print(f"🔍 Executing SPARQL on '{namespace}'...")
        res = stub.QuerySparql(req)
        
        # Parse and pretty print JSON results
        try:
            results = json.loads(res.results_json)
            print(json.dumps(results, indent=2))
        except:
            print(res.results_json)
            
    except grpc.RpcError as e:
        print(f"❌ gRPC Error: {e.details()}")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Execute SPARQL query on Synapse")
    parser.add_argument("query", help="SPARQL query string")
    parser.add_argument("--namespace", default="robin_os", help="Namespace to query")
    
    args = parser.parse_args()
    query_sparql(args.query, args.namespace)
