# Synapse 🧠⛓️

**Synapse** is a high-performance, purely symbolic knowledge graph system designed to serve as the long-term, determinable memory for AI agents.

Built entirely in Rust, Synapse relies on a formal logical reasoning core instead of probabilistic vectors. This guarantees absolute certainty in data retrieval, provenance tracking, and deductive reasoning capabilities.

## 🚀 Key Capabilities

-   **Blazing Fast Core**: Powered by Rust and [Oxigraph](https://github.com/oxigraph/oxigraph) for low-latency, strictly symbolic graph operations.
-   **Markdown-Graph Symbiosis**: Bidirectionally syncs human-readable Markdown files (via YAML frontmatter and WikiLinks) with the active RDF graph.
-   **Reasoning Engine**: In-memory RDFS/OWL-RL fixed-point reasoner (`reasoner.rs`). *Not yet exposed through MCP* — it is not invoked by any tool today.
-   **Scenario Marketplace**: Dynamic loading of domain-specific "scenarios" (ontologies + seed data) to instantly equip agents with specialized schemas.
-   **Native MCP & UI Extensions**: Seamlessly integrates as a [Model Context Protocol](https://modelcontextprotocol.io) server, supporting dual-transport (HTTP/SSE or Standard I/O) and rich Ext-App HTML/d3 UI rendering.
-   **Ontology-Driven**: Loads the ontologies of the scenarios listed in `SYNAPSE_SCENARIOS` (default `core`: Schema.org, PROV-O, SKOS, FOAF, Memory) at startup; unchanged files are skipped.

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
- `sparql_query`: Execute a SPARQL query. The default graph is the union of all graphs (unless the query sets `FROM`).
- `assert_facts`: Record facts as one assertion with provenance (source, reason, time). Returns an assertion id.
- `retract_facts`: Retract a whole assertion or individual asserted facts; every retraction is logged.
- `index_markdown_directory`: Index or re-index a folder of Markdown files (e.g. an OKF bundle). Changed files replace their facts, deleted files drop them.
- `sync_specification_to_graph`: Ingest a single Markdown file tagged as a core specification.
- `get_entity_narrative`: Markdown dossier of an entity (attributes, outbound and inbound relations).
- `get_entity_neighborhood`: Expand the graph around an entity (plus a `ui://` view).
- `get_domain_logic`: Triples of every class under a given root class (`rdfs:subClassOf*`).
- `get_dependency_impact`: Entities that point (transitively, up to `depth`) at a component.
- `get_provenance`: Provenance metadata of an ingestion batch.

#### Available MCP Resources (Ext-Apps):
- `ui://synapse/dashboard`: Returns an interactive HTML dashboard view of the engine metrics.
- `ui://synapse/graph/{uri}`: Returns a D3.js powered subgraph visualization for the requested entity.

## 📚 Scenario Architecture

Synapse enforces structural integrity via **Scenarios**. A Scenario bundles schemas and data:
1.  **Ontologies**: Formal schema definitions (OWL/TTL) that define classes and property chains.
2.  **Seed Data**: Initial foundational knowledge triples.

### Built-in Scenarios:
*   **Core**: Essential ontologies (Schema.org, PROV-O, SKOS, FOAF, Memory).
*   **Research Assistant**: Specialized ontologies designed for tracking academic papers and citations.

## 🏗️ Technical Architecture

### 1. Purely Symbolic Graph
All probabilistic ML (embeddings, vectors, fastembed) has been explicitly purged. The system relies 100% on explicit RDF triples, ensuring zero hallucinations in memory recall.

### 2. The Synapse Reasoner
`reasoner.rs` implements a fixed-point materialization loop (not yet wired to any MCP tool):
*   **RDFS**: `rdfs:subClassOf` transitivity.
*   **OWL-RL**: `owl:SymmetricProperty`, `owl:TransitiveProperty`, `owl:inverseOf` and a binary property-chain shortcut (`urn:synapse:chainFirst` / `urn:synapse:chainSecond`).

### 3. Identity, Provenance & Retraction
*   **Stable identity**: an entity id is the path of its Markdown file relative to the indexed folder, without extension (`people/bob.md` → `people/bob` → `urn:synapse:entity:people/bob`), or an explicit `uri:` in the frontmatter. `[[people/bob]]` links, agent assertions about `people/bob` and the file itself are the same node. Relations default to `urn:synapse:prop:<name>`, classes to `urn:synapse:type:<Name>`; known prefixes (`rdf:`, `rdfs:`, `owl:`, `xsd:`, `prov:`, `schema:`, `skos:`, `foaf:`) and full IRIs are kept.
*   **One named graph per origin**: each Markdown file, ontology or CSV owns a source graph; each `assert_facts` call owns an assertion graph. Their PROV-O metadata (`prov:wasDerivedFrom`, `prov:generatedAtTime`, `prov:wasAttributedTo`, content hash) lives in `urn:synapse:meta`.
*   **Retraction**: re-indexing a source replaces its graph, so edits and deletions are reflected; asserted facts are removed with `retract_facts`, which logs what was removed, by whom and why.

### 4. Markdown format (OKF-compatible)
```markdown
---
type: person          # -> rdf:type urn:synapse:type:person
title: Alice          # -> rdfs:label and urn:synapse:prop:title
tags: [staff, rust]   # lists -> one literal each
---
Works with [[people/bob]] and [[depends_on:libs/sqlx]] (also [[rel::target]], [[target|alias]]).
```
Synapse maintains a `## 🧠 Synapse Backlinks` section at the end of each indexed file with its inbound links; that section is ignored when parsing.

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on how to get started.

## ⚖️ License

This project is licensed under the **MIT License** - see the [LICENSE](LICENSE) file for details.

---
*Developed by the Synapse Team*
