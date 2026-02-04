import sys
import os
import json
import logging

# Setup path
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '../agents/infrastructure/web')))

try:
    from agents.infrastructure.web.client import get_client
except ImportError as e:
    # Fallback: try importing client directly if path messiness occurs
    try:
        sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '../agents/infrastructure/web')))
        from client import get_client
    except ImportError as e2:
        print(f"Error importing client: {e} | {e2}")
        sys.exit(1)

def main():
    print("🚀 Starting ingestion...")
    
    # Load triples
    triples_path = os.path.join(os.path.dirname(__file__), '../data/triples_to_ingest.json')
    if not os.path.exists(triples_path):
        print("No triples file found.")
        sys.exit(0)
        
    with open(triples_path, 'r') as f:
        triples = json.load(f)
        
    client = get_client()
    if not client.connect():
        print("Failed to connect to Synapse.")
        sys.exit(1)
        
    print(f"Ingesting {len(triples)} triples into namespace 'work'...")
    result = client.ingest_triples(triples, namespace="work")
    print(f"Result: {result}")
    
    # Clear buffer
    buffer_path = os.path.join(os.path.dirname(__file__), '../data/ingest_buffer.json')
    if os.path.exists(buffer_path):
        print("Clearing buffer...")
        with open(buffer_path, 'w') as f:
            json.dump([], f)
        print("Buffer cleared.")

    print("Done.")

if __name__ == "__main__":
    main()
