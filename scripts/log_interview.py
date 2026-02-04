import grpc
import sys
import os
from datetime import datetime

# Setup paths
WORKSPACE = "/home/robin/workspace/skills/synapse"
sys.path.append(WORKSPACE)
sys.path.append(os.path.join(WORKSPACE, "agents/infrastructure/web"))

import agents.infrastructure.web.semantic_engine_pb2 as pb2
import agents.infrastructure.web.semantic_engine_pb2_grpc as pb2_grpc

def log_event():
    address = "localhost:50051"
    channel = grpc.insecure_channel(address)
    stub = pb2_grpc.SemanticEngineStub(channel)
    
    # Event: Interview with Cabify
    triples = [
        pb2.Triple(subject="Interview_Cabify_DesignManager", predicate="rdf:type", object="Event"),
        pb2.Triple(subject="Interview_Cabify_DesignManager", predicate="hasParticipant", object="Pelayo"),
        pb2.Triple(subject="Interview_Cabify_DesignManager", predicate="scheduledAt", object="2026-02-05T13:00:00+01:00"),
        pb2.Triple(subject="Interview_Cabify_DesignManager", predicate="role", object="Web Design Manager"),
        pb2.Triple(subject="Cabify", predicate="isA", object="Company"),
        pb2.Triple(subject="Interview_Cabify_DesignManager", predicate="company", object="Cabify"),
    ]
    
    try:
        req = pb2.IngestRequest(triples=triples, namespace="personal")
        res = stub.IngestTriples(req)
        print(f"✅ Ingested {res.edges_added} facts about the Cabify interview into Synapse.")
    except Exception as e:
        print(f"❌ Error: {e}")

if __name__ == "__main__":
    log_event()
