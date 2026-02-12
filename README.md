# Synapse 🧠⛓️

**Synapse** is a high-performance, neuro-symbolic knowledge graph system designed to serve as the long-term memory for agentic AI. It bridges the gap between **unstructured semantic search (Vector RAG)** and **formal logical reasoning (Knowledge Graphs)**.

## 🚀 Key Capabilities

-   **Blazing Fast Core**: Powered by Rust and [Oxigraph](https://github.com/oxigraph/oxigraph) for low-latency graph operations.
-   **Neuro-symbolic Search**: Hybrid retrieval combining vector similarity with graph traversal expansion.
-   **Reasoning Engine**: Built-in OWL-RL and RDFS reasoning strategies to derive implicit knowledge.
-   **Multi-Tenancy**: Native support for isolated namespaces (e.g., `work`, `personal`, `os`).
-   **Native MCP**: Seamlessly integrates as a [Model Context Protocol](https://modelcontextprotocol.io) server.
-   **Ontology-Driven**: Automatically loads standard ontologies (Schema.org, PROV-O, etc.) to structure knowledge.

## 📦 Installation & Setup

### One-Click for OpenClaw
```bash
npx skills install pmaojo/synapse-engine
```
*Note: During installation, you will be prompted to set Synapse as your default memory provider.*

### Python SDK
v0.5.2 introduces the official high-level SDK:
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

### MCP Integration (v0.5.2)
Add Synapse to your `openclaw.json` (or Cursor/Claude Desktop) to enable direct LLM access to your knowledge graph:

```json
"mcpServers": {
  "synapse": {
    "command": "path/to/synapse",
    "args": ["--mcp"],
    "env": { 
      "GRAPH_STORAGE_PATH": "./data/graphs" 
    }
  }
}
```

#### Available Tools:
- `ingest_triples`: Direct RDF ingestion.
- `sparql_query`: Complex graph querying.
- `hybrid_search`: Semantic + structural retrieval.
- `apply_reasoning`: Trigger OWL-RL/RDFS inference.
- `ingest_url`: Automated scraping and embedding.
- `install_ontology`: Download and install new ontologies from the web.

## 📚 Ontologies & Semantic Memory

Synapse comes pre-loaded with essential ontologies to help Agents understand the world "out-of-the-box":

*   **Schema.org** (`schema.owl`): Core definitions for Person, Action, Event, Organization.
*   **PROV-O** (`prov.owl`): Provenance ontology to track where knowledge comes from (e.g., `wasDerivedFrom`, `generatedAtTime`).
*   **Memory Ontology** (`memory.owl`): Specialized for Episodic Memory (`Conversation`, `UserInstruction`, `AgentAction`).
*   **SKOS** (`skos.owl`): Simple Knowledge Organization System for concepts and tesaurus.
*   **FOAF** (`foaf.owl`): Friend of a Friend for social connections.

These files are located in `ontology/` and are automatically loaded into the `default` namespace on startup.

To add a new ontology dynamically via MCP:
```json
{
  "name": "install_ontology",
  "arguments": {
    "url": "https://raw.githubusercontent.com/ad-hoc-network/legal-ontology/main/legal.owl",
    "name": "legal.owl"
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

## 🧪 Testing

Synapse includes an E2E test suite to verify the integration between the Python client and Rust backend.

```bash
# Ensure Rust server is running
./start_rust_server.sh

# Run tests
pytest tests/
```

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on how to get started.

## ⚖️ License

This project is licensed under the **MIT License** - see the [LICENSE](LICENSE) file for details.

---
*Developed by Pelayo Maojo & the Synapse Team*
