# Synapse 🧠⛓️

**Synapse** is a high-performance, neuro-symbolic knowledge graph system designed to serve as the long-term memory for agentic AI. It bridges the gap between **unstructured semantic search (Vector RAG)** and **formal logical reasoning (Knowledge Graphs)**.

## 🚀 Key Capabilities

-   **Blazing Fast Core**: Powered by Rust and [Oxigraph](https://github.com/oxigraph/oxigraph) for low-latency graph operations.
-   **Neuro-symbolic Search**: Hybrid retrieval combining vector similarity with graph traversal expansion.
-   **Reasoning Engine**: Built-in OWL-RL and RDFS reasoning strategies to derive implicit knowledge.
-   **Multi-Tenancy**: Native support for isolated namespaces (e.g., `work`, `personal`, `os`).
-   **Native MCP**: Seamlessly integrates as a [Model Context Protocol](https://modelcontextprotocol.io) server.

## 📦 Installation & Setup

### One-Click for OpenClaw
```bash
npx skills install pmaojo/synapse-engine
```

### Python SDK
v0.4.0 introduces the official high-level SDK:
```bash
pip install ./python-sdk
```

## 🛠️ Usage

### Python SDK (Recommended)
Connect and ingest knowledge with just a few lines of code:

```python
from synapse import get_client

# Connect to local engine
client = get_client()

# Ingest semantic triples
client.ingest_triples([
    {"subject": "Pelayo", "predicate": "expertIn", "object": "Neuro-symbolic AI"}
], namespace="work")

# Hybrid Search
results = client.hybrid_search("What is Pelayo's expertise?", namespace="work")
```

### MCP Integration
Add Synapse to your `openclaw.json` (or Cursor/Claude Desktop) to enable direct LLM access:

```json
"mcpServers": {
  "synapse": {
    "command": "synapse",
    "args": ["--mcp"],
    "env": { "GRAPH_STORAGE_PATH": "./data/graphs" }
  }
}
```

## 🌐 Notion Sync: Automated Memory

Synapse can automatically distill your Notion notes into formal knowledge using LLM-driven extraction.

1. Configure the `notion` skill in your environment.
2. Add a sync job to your `openclaw.json`:
```bash
openclaw cron add --name "Notion Sync" --every "1h" --message "Sync recent Notion pages to Synapse namespace 'personal'"
```

## 🏗️ Technical Architecture

### 1. Ontology-Driven Validation
Ontologies are defined in standard OWL format (`ontology/*.owl`). Synapse uses these schemas to validate incoming triples, ensuring semantic consistency (domain/range checks) and preventing logical contradictions.

### 2. The Synapse Reasoner
The Rust core implements a multi-strategy reasoner:
*   **RDFS**: Efficient class and property transitivity.
*   **OWL-RL**: Advanced logic for `SymmetricProperty`, `TransitiveProperty`, and `inverseOf` relationships.
*   **Materialization**: Inferred facts are persisted in the graph, making reasoning-based queries near-instantaneous.

### 3. Robust Ingestion
v0.4.0 includes a new **Rollback Mechanism**: if vector indexing fails during ingestion, graph changes are automatically reverted to maintain memory integrity.

---
*Developed by Pelayo Maojo & the Synapse Team*
