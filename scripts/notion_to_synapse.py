import sys
import os
import json
from typing import List, Dict

# Path setup for Notion skill
SKILLS_PATH = "/home/robin/workspace/skills/pm-comm/notion"
sys.path.append(SKILLS_PATH)

from notion_client import NotionClient

def fetch_recent_notes():
    client = NotionClient(config_path="/home/robin/workspace/.config/notion/config.json")
    
    print("🔍 Fetching recent notes from Notion...")
    try:
        # Query memory database
        # Note: notion_client uses 'data_sources' endpoint which might be internal/custom
        # or it might be the standard 'databases' endpoint. 
        # In notion_client.py it says: f"data_sources/{db_id}/query"
        # I'll stick to what the client expects.
        results = client.query_database("memory")
        
        notes = []
        for page in results:
            properties = page.get("properties", {})
            name = properties.get("Name", {}).get("title", [{}])[0].get("plain_text", "Untitled")
            # For simplicity, we just take the title and some metadata for now
            # Fetching full content would require block-by-block retrieval
            notes.append({
                "title": name,
                "id": page["id"],
                "url": page.get("url")
            })
            
        return notes
    except Exception as e:
        print(f"❌ Failed to fetch from Notion: {e}")
        return []

def main():
    notes = fetch_recent_notes()
    if not notes:
        print("No new notes found.")
        return

    # Write to a buffer for Robin to process
    buffer_path = "/home/robin/workspace/synapse/data/ingest_buffer.json"
    with open(buffer_path, 'w') as f:
        json.dump(notes, f, indent=2)
    
    print(f"✅ Stored {len(notes)} notes in {buffer_path} for Synapse distillation.")

if __name__ == "__main__":
    main()
