import os
import sys
import re
import grpc
from synapse import get_client

# Add generated stubs to path for direct Triple construction if needed
# though SDK handles most
sys.path.append("/home/robin/workspace/skills/synapse/scripts/generated")
try:
    import semantic_engine_pb2 as pb2
except ImportError:
    pb2 = None

# Add notion client to path
sys.path.append("/home/robin/workspace/skills/pm-comm/notion")
from notion_client import NotionClient

def populate():
    print("🧠 Synapse Librarian: Starting full system sync...")
    client = get_client()
    
    # Use admin token from env
    token = os.getenv("SYNAPSE_ADMIN_TOKEN", "admin_token")
    
    # 1. Sync Local Files (MEMORY.md + Logs)
    print("📁 Syncing local files...")
    memory_path = "/home/robin/workspace/MEMORY.md"
    if os.path.exists(memory_path):
        with open(memory_path, 'r') as f:
            content = f.read()
        insights = re.findall(r'- (\d{4}-\d{2}-\d{2}): (.+)', content)
        triples = []
        for date, text in insights:
            entry_id = f"memory:insight:{abs(hash(text))}"
            triples.append({"subject": entry_id, "predicate": "dc:date", "object": f"\"{date}\""})
            triples.append({"subject": entry_id, "predicate": "dc:description", "object": f"\"{text}\""})
        if triples:
            client.ingest_triples(triples, namespace="os")

    # 2. Sync Notion (Memory + Tasks)
    print("🌐 Syncing Robin OS from Notion...")
    try:
        notion = NotionClient(config_path="/home/robin/workspace/.config/notion/config.json")
        
        # A. Sync Memory/Insights
        notion_notes = notion.query_database("memory")
        note_triples = []
        for page in notion_notes:
            props = page.get("properties", {})
            title = props.get("Name", {}).get("title", [{}])[0].get("plain_text", "Untitled")
            note_id = page["id"]
            note_triples.append({
                "subject": f"notion:note:{note_id}",
                "predicate": "dc:title",
                "object": f"\"{title}\""
            })
            note_triples.append({
                "subject": f"notion:note:{note_id}",
                "predicate": "rdf:type",
                "object": "schema:Note"
            })
        if note_triples:
            client.ingest_triples(note_triples, namespace="os")
            print(f"✅ Ingested {len(note_triples)//2} notes from Notion.")

        # B. Sync Tasks
        notion_tasks = notion.query_database("tasks")
        task_triples = []
        for page in notion_tasks:
            props = page.get("properties", {})
            title = props.get("Task name", {}).get("title", [{}])[0].get("plain_text", "Untitled")
            status = props.get("Status", {}).get("status", {}).get("name", "Unknown")
            task_id = page["id"]
            task_triples.append({
                "subject": f"notion:task:{task_id}",
                "predicate": "dc:title",
                "object": f"\"{title}\""
            })
            task_triples.append({
                "subject": f"notion:task:{task_id}",
                "predicate": "synapse:status",
                "object": f"\"{status}\""
            })
        if task_triples:
            client.ingest_triples(task_triples, namespace="os")
            print(f"✅ Ingested {len(task_triples)//2} tasks from Notion.")

    except Exception as e:
        print(f"⚠️ Notion sync failed: {e}")

    print("✅ Synapse Librarian: System sync complete.")

if __name__ == "__main__":
    populate()
