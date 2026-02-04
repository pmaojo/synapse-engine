import sys
import os
import json
import logging
import grpc

# Add generated stubs to path
sys.path.append(os.path.join(os.path.dirname(__file__), 'generated'))
try:
    import semantic_engine_pb2
    import semantic_engine_pb2_grpc
except ImportError:
    print("Could not import generated stubs.")
    sys.exit(1)

# Add notion client to path
sys.path.append("/home/robin/workspace/skills/pm-comm/notion")
try:
    from notion_client import NotionClient
except ImportError:
    print("Could not import notion_client.")
    sys.exit(1)

def fetch_recent_notes(client):
    print("🔍 Fetching recent notes from Notion...")
    try:
        results = client.query_database("memory")
        notes = []
        for page in results:
            properties = page.get("properties", {})
            name = properties.get("Name", {}).get("title", [{}])[0].get("plain_text", "Untitled")
            page_id = page["id"]
            url = page.get("url")
            notes.append({
                "title": name,
                "id": page_id,
                "url": url
            })
        return notes
    except Exception as e:
        print(f"❌ Failed to fetch from Notion: {e}")
        return []

def main():
    try:
        client = NotionClient(config_path="/home/robin/workspace/.config/notion/config.json")
    except Exception as e:
        print(f"Failed to init Notion client: {e}")
        return

    notes = fetch_recent_notes(client)
    
    if not notes:
        print("No notes found.")
        return

    print(f"Connecting to Synapse gRPC...")
    channel = grpc.insecure_channel('localhost:50051')
    stub = semantic_engine_pb2_grpc.SemanticEngineStub(channel)
    
    triples = []
    for note in notes:
        subject = f"notion:{note['id']}"
        provenance = f"notion_sync:{note['url']}"
        
        # Title
        triples.append(semantic_engine_pb2.Triple(
            subject=subject,
            predicate="http://purl.org/dc/terms/title",
            object=f"\"{note['title']}\"",
            provenance=semantic_engine_pb2.Provenance(source=provenance, method="notion_sync")
        ))
        
        # Type
        triples.append(semantic_engine_pb2.Triple(
            subject=subject,
            predicate="http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
            object="http://schema.org/Note",
            provenance=semantic_engine_pb2.Provenance(source=provenance, method="notion_sync")
        ))

    req = semantic_engine_pb2.IngestRequest(
        triples=triples,
        namespace="robin_os"
    )
    
    try:
        res = stub.IngestTriples(req)
        print(f"✅ Successfully ingested {res.nodes_added} triples.")
    except grpc.RpcError as e:
        print(f"❌ gRPC Error: {e.code()} - {e.details()}")

if __name__ == "__main__":
    main()
