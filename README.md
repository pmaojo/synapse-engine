# Synapse 🧠⛓️

**Synapse** is a high-performance, neuro-symbolic knowledge graph system designed to serve as the long-term memory for agentic AI. It bridges the gap between **unstructured semantic search (Vector RAG)** and **formal logical reasoning (Knowledge Graphs)**.

## 🚀 Key Capabilities

-   **Blazing Fast Core**: Powered by Rust and [Oxigraph](https://github.com/oxigraph/oxigraph) for low-latency graph operations.
-   **Neuro-symbolic Search**: Hybrid retrieval combining vector similarity with graph traversal expansion.
-   **Reasoning Engine**: Built-in OWL-RL and RDFS reasoning strategies to derive implicit knowledge.
-   **Scenario Marketplace**: (v0.6.0) Dynamic loading of domain-specific "scenarios" (ontologies + data + docs) to instantly equip agents with specialized knowledge.
-   **Native MCP**: Seamlessly integrates as a [Model Context Protocol](https://modelcontextprotocol.io) server.
-   **Ontology-Driven**: Automatically loads standard ontologies (Schema.org, PROV-O, etc.) via the `core` scenario.

## 📦 Installation & Setup

### One-Click for OpenClaw
```bash
npx skills install pmaojo/synapse-engine
```
*Note: During installation, you will be prompted to set Synapse as your default memory provider.*

### Python SDK
v0.6.0 introduces the official high-level SDK:
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
- `list_scenarios`: Browse the Scenario Marketplace.
- `install_scenario`: Install a domain package (e.g., `research-assistant`).
- `ingest_triples`: Direct RDF ingestion.
- `sparql_query`: Complex graph querying.
- `hybrid_search`: Semantic + structural retrieval.
- `apply_reasoning`: Trigger OWL-RL/RDFS inference.
- `ingest_url`: Automated scraping and embedding.

## 📚 Scenario Marketplace (New in v0.6.0)

Synapse now supports a **Scenario Marketplace**, allowing agents to dynamically install knowledge packages. A Scenario bundles:
1.  **Ontologies**: Formal schema definitions (OWL).
2.  **Seed Data**: Initial knowledge graph triples.
3.  **Documentation**: Text guides automatically indexed for RAG retrieval.

### Built-in Scenarios:
*   **Core**: Essential ontologies (Schema.org, PROV-O, SKOS, FOAF, Memory) loaded by default.
*   **Research Assistant**: Specialized ontology for academic papers and authors.

To install a scenario via MCP:
```json
{
  "name": "install_scenario",
  "arguments": {
    "name": "research-assistant",
    "namespace": "my-research"
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
Ontologies are defined in standard OWL format. Synapse uses these schemas to validate incoming triples, ensuring semantic consistency (domain/range checks) and preventing logical contradictions.

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
