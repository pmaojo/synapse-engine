# Synapse vs MemPalace: A Feature Comparison

Both **Synapse** and **MemPalace** are designed to provide long-term memory solutions for AI agents, but they take fundamentally different architectural approaches.

**Synapse** is a purely symbolic, deterministic knowledge graph built in Rust, relying on formal ontologies, RDF triples, and logical reasoning (OWL-RL, RDFS) to ensure absolute certainty and traceability.

**MemPalace** is a Python-based system built around vector search (ChromaDB) and an SQLite entity-relationship graph, focusing on a structured hierarchical metaphor (Palaces, Wings, Rooms) and experimental text abbreviation (AAAK) to maximize LLM context windows.

## Feature Comparison Matrix

| Feature | Synapse 🧠⛓️ | MemPalace 🏛️ |
| :--- | :--- | :--- |
| **Core Architecture** | Purely Symbolic Knowledge Graph (RDF/SPARQL) | Semantic Search (Vector) + Relational (SQLite) |
| **Primary Language** | Rust (High performance, memory safe) | Python |
| **Storage Engine** | Oxigraph (Symbolic RDF graph) | ChromaDB (Vectors) & SQLite (Knowledge Graph) |
| **Graph Type** | Standardized RDF Triples (Named Graphs) | Custom Temporal Entity-Relationship Triples |
| **Data Structure** | Ontologies (Schema.org, PROV-O, etc.), Scenarios, Markdown-Graph Symbiosis | Hierarchical Metaphor (Wings, Halls, Rooms, Closets, Drawers) |
| **Reasoning Engine** | Built-in formal OWL-RL and RDFS fixed-point reasoning | None (relies on semantic search and separate fact checker utility) |
| **Data Retrieval** | Deterministic SPARQL queries and k-hop neighborhood expansion | Vector similarity search with metadata filtering |
| **Memory Optimization** | Contextual Subgraph extraction (k-hop expansion) | AAAK (Experimental lossy abbreviation dialect) |
| **Data Ingestion** | Markdown syncing, explicit `sparql_update` | Directory mining, conversation parsing (`mempalace mine`) |
| **Provenance Tracking** | Native via PROV-O (`prov:wasDerivedFrom`) & Named Graphs | Temporal validity windows (`valid_from`, `ended`) |
| **LLM Integration (MCP)** | Yes (Dual-transport: stdio & HTTP/SSE, Ext-App UI resources) | Yes (19 tools via stdio) |
| **Probabilistic ML/Vectors**| **None** (Explicitly purged to prevent hallucinations) | **Core** (Relies heavily on ChromaDB vector search) |

## Architectural Philosophies

### Synapse: Determinism and Formal Logic
Synapse is built for environments where truth, provenance, and deterministic recall are non-negotiable.
- **Zero Hallucination Retrieval:** By avoiding vector embeddings entirely, Synapse ensures that what the agent retrieves is exactly what was stored.
- **Formal Ontologies:** It uses established semantic web standards (OWL, RDF, Schema.org). This allows for complex, standardized inferences (e.g., if A is a subclass of B, and X is type A, the system *knows* X is type B without needing an LLM to guess).
- **Extensibility:** Scenarios allow dynamic loading of domain-specific ontologies, instantly equipping an agent with new structural schemas.

### MemPalace: Heuristic Retrieval and Context Packing
MemPalace is built around maximizing the efficiency of the LLM context window using heuristic structures.
- **Spatial Metaphor:** It forces data into a rigid hierarchy (Wings for projects/people, Halls for memory types, Rooms for topics). This metadata filtering significantly boosts ChromaDB's retrieval accuracy.
- **AAAK Compression:** It experiments with a lossy, custom text abbreviation dialect to compress memories so more context can fit into the LLM's prompt window.
- **Verbatim Storage:** It stores exact conversation transcripts in ChromaDB "drawers" rather than relying on LLM summarization upfront.

## Conclusion

Choose **Synapse** if you are building autonomous systems that require formal logical deduction, 100% deterministic memory recall, bidirectional syncing with human-readable Markdown, and a high-performance Rust foundation.

Choose **MemPalace** if you are looking for a Python-based utility that excels at indexing massive raw chat transcripts, and you want to leverage vector search augmented by strict metadata categorization (Wings/Rooms) for context retrieval.
