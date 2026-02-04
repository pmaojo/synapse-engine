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

def apply_reasoning(namespace="robin_os", strategy="OWLRL", materialize=True):
    channel = grpc.insecure_channel('localhost:50051')
    stub = pb2_grpc.SemanticEngineStub(channel)
    
    strat_enum = pb2.ReasoningStrategy.OWLRL
    if strategy.upper() == "RDFS":
        strat_enum = pb2.ReasoningStrategy.RDFS
    elif strategy.upper() == "NONE":
        strat_enum = pb2.ReasoningStrategy.NONE
        
    req = pb2.ReasoningRequest(
        namespace=namespace,
        strategy=strat_enum,
        materialize=materialize
    )
    
    try:
        print(f"🧠 Applying {strategy} reasoning on namespace '{namespace}'...")
        res = stub.ApplyReasoning(req)
        if res.success:
            print(f"✅ Reasoning complete. Inferred {res.triples_inferred} new facts.")
            print(f"Message: {res.message}")
        else:
            print(f"❌ Reasoning failed: {res.message}")
    except grpc.RpcError as e:
        print(f"❌ gRPC Error: {e.details()}")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Trigger reasoning on Synapse")
    parser.add_argument("--namespace", default="robin_os", help="Namespace to reason over")
    parser.add_argument("--strategy", default="OWLRL", choices=["OWLRL", "RDFS", "NONE"], help="Reasoning strategy")
    parser.add_argument("--no-materialize", action="store_false", dest="materialize", help="Do not save inferred triples")
    
    args = parser.parse_args()
    apply_reasoning(args.namespace, args.strategy, args.materialize)
