# Synapse 🧠⛓️

**Synapse** is a high-performance, purely symbolic knowledge graph system designed to serve as the long-term, determinable memory for AI agents.

Built entirely in Rust, Synapse relies on a formal logical reasoning core instead of probabilistic vectors. This guarantees absolute certainty in data retrieval, provenance tracking, and deductive reasoning capabilities.

## 🚀 Key Capabilities

-   **Blazing Fast Core**: Powered by Rust and [Oxigraph](https://github.com/oxigraph/oxigraph) for low-latency, strictly symbolic graph operations.
-   **Markdown-Graph Symbiosis**: Bidirectionally syncs human-readable Markdown files (via YAML frontmatter and WikiLinks) with the active RDF graph.
-   **Reasoning Engine**: Built-in, in-memory OWL-RL and RDFS fixed-point reasoning strategies to derive implicit knowledge safely.
-   **Scenario Marketplace**: Dynamic loading of domain-specific "scenarios" (ontologies + seed data) to instantly equip agents with specialized schemas.
-   **Native MCP & UI Extensions**: Seamlessly integrates as a [Model Context Protocol](https://modelcontextprotocol.io) server, supporting dual-transport (HTTP/SSE or Standard I/O) and rich Ext-App HTML/d3 UI rendering.
-   **Ontology-Driven**: Automatically loads standard ontologies (Schema.org, PROV-O, SKOS, etc.) via the `core` scenario.

## 📦 Installation & Setup

Ensure you have [Rust](https://rustup.rs/) installed, then build the release binary:

```bash
git clone https://github.com/pmaojo/synapse-engine.git
cd synapse-engine
./start_rust_server.sh
```

## 🛠️ Usage

### MCP Integration (Claude Desktop / Cursor)
Add Synapse to your MCP client configuration to enable direct LLM access to the symbolic knowledge graph:

```json
"mcpServers": {
  "synapse": {
    "command": "/path/to/synapse-engine/target/release/synapse",
    "args": ["--stdio"],
    "env": { 
      "GRAPH_STORAGE_PATH": "/path/to/synapse-engine/data/graphs"
    }
  }
}
```

#### Available MCP Tools:
- `sparql_query`: Execute strict, graph-based SPARQL queries to traverse deterministic memory.
- `index_markdown_directory`: Recursively map a folder of Markdown files into the RDF graph.
- `get_entity_neighborhood`: Extract the immediate surrounding entities (1-hop) for contextual retrieval.

#### Available MCP Resources (Ext-Apps):
- `ui://synapse/dashboard`: Returns an interactive HTML dashboard view of the engine metrics.
- `ui://synapse/graph/{uri}`: Returns a D3.js powered subgraph visualization for the requested entity.

## 📚 Scenario Architecture

Synapse enforces structural integrity via **Scenarios**. A Scenario bundles schemas and data:
1.  **Ontologies**: Formal schema definitions (OWL/TTL) that define classes and property chains.
2.  **Seed Data**: Initial foundational knowledge triples.

### Built-in Scenarios:
*   **Core**: Essential ontologies (Schema.org, PROV-O, SKOS, FOAF, Memory) loaded automatically at startup.
*   **Research Assistant**: Specialized ontologies designed for tracking academic papers and citations.

## 🏗️ Technical Architecture

### 1. Purely Symbolic Graph
All probabilistic ML (embeddings, vectors, fastembed) has been explicitly purged. The system relies 100% on explicit RDF triples, ensuring zero hallucinations in memory recall.

### 2. The Synapse Reasoner
The engine implements a multi-strategy materialization loop:
*   **RDFS**: Efficient class (`rdfs:subClassOf`) and property (`rdfs:subPropertyOf`) transitivity.
*   **OWL-RL**: Deterministic fixed-point inference for axioms like `owl:SymmetricProperty`, `owl:TransitiveProperty`, and `owl:inverseOf`.

### 3. Provenance & Integrity
Every fact ingested into the graph is stored within an isolated Named Graph, linked via PROV-O properties (`prov:wasDerivedFrom`, `prov:generatedAtTime`). You always know exactly *where* and *when* an agent learned a piece of information.

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on how to get started.

## ⚖️ License

This project is licensed under the **MIT License** - see the [LICENSE](LICENSE) file for details.

---
*Developed by the Synapse Team*
