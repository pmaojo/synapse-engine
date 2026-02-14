# Synapse Core 🧠

<div align="center">

[![Crates.io](https://img.shields.io/crates/v/synapse-core.svg)](https://crates.io/crates/synapse-core)
[![Documentation](https://docs.rs/synapse-core/badge.svg)](https://docs.rs/synapse-core)
[![License](https://img.shields.io/crates/l/synapse-core.svg)](https://github.com/pmaojo/synapse-engine/blob/main/LICENSE)

**A high-performance neuro-symbolic semantic engine designed for agentic AI.**

[Features](#-features) • [Installation](#-installation) • [Usage](#-usage) • [API Reference](#-api-reference) • [Architecture](#-architecture)

</div>

---

## 📖 Overview

**Synapse Core** provides the foundational semantic memory layer for AI agents. It combines the structured precision of **Knowledge Graphs** (using [Oxigraph](https://github.com/oxigraph/oxigraph)) with **RDF/SPARQL** standards, allowing agents to reason about data, maintain long-term context, and query knowledge using industry-standard semantic web technologies.

It is designed to work seamlessly with **OpenClaw** and other agentic frameworks via the **Model Context Protocol (MCP)** or as a standalone **gRPC service**.

## 🚀 Features

- **RDF Triple Store**: Built on Oxigraph for standards-compliant RDF storage and querying
- **SPARQL Support**: Full SPARQL 1.1 query language support for complex graph queries
- **Multi-Namespace Architecture**: Isolated knowledge bases for different contexts (work, personal, projects)
- **Dual Protocol Support**:
  - **gRPC API** for high-performance programmatic access
  - **MCP Server** for seamless LLM agent integration
- **OWL Reasoning**: Built-in support for OWL 2 RL reasoning via `reasonable` crate
- **Hybrid Search**: Combines vector similarity with graph traversal (using local HNSW index)
- **HuggingFace API Integration**: High-performance embeddings without local GPU/CPU heavy lifting
- **High Performance**: Written in Rust with async I/O and efficient HNSW indexing
- **Persistent Storage**: Automatic persistence with namespace-specific storage paths
- **Granular Security**: Token-based authorization for Read, Write, Delete, and Reason operations.
- **Robust MCP**: Strict JSON Schema validation for all Model Context Protocol tool calls.

## 📦 Installation

### As a Rust Library

Add to your `Cargo.toml`:

```toml
[dependencies]
synapse-core = "0.8.4"
```

### As a Binary

Install the CLI tool:

```bash
cargo install synapse-core
```

### For OpenClaw

One-click install as an MCP server:

```bash
npx skills install pmaojo/synapse-engine
```

## 🛠️ Usage

### 1. Standalone gRPC Server

Run Synapse as a high-performance gRPC server:

```bash
# Start the server (default: localhost:50051)
synapse

# With custom storage path
GRAPH_STORAGE_PATH=/path/to/data synapse
```

The gRPC server exposes 7 RPC methods for semantic operations (see [API Reference](#-api-reference)).

### 2. Model Context Protocol (MCP) Server

Run in MCP mode for integration with LLM agents:

```bash
synapse --mcp
```

This exposes 3 MCP tools via JSON-RPC over stdio:

- `query_graph` - Retrieve all triples from a namespace
- `ingest_triple` - Add a new triple to the knowledge graph
- `query_sparql` - Execute SPARQL queries

### 3. Rust Library Integration

Embed the engine directly into your application:

```rust
use synapse_core::server::MySemanticEngine;
use synapse_core::server::semantic_engine::*;
use tonic::Request;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the engine
    let engine = MySemanticEngine::new("data/my_graph");

    // Ingest triples
    let triple = Triple {
        subject: "Alice".to_string(),
        predicate: "knows".to_string(),
        object: "Bob".to_string(),
        provenance: None,
    };

    let request = IngestRequest {
        triples: vec![triple],
        namespace: "social".to_string(),
    };

    let response = engine.ingest_triples(Request::new(request)).await?;
    println!("Added {} triples", response.into_inner().nodes_added);

    Ok(())
}
```

### 4. Hybrid Search

Retrieve entities matching both semantic similarity (vector) and structural relationship (graph):

```rust
use synapse_core::server::proto::{HybridSearchRequest, SearchMode};

let request = HybridSearchRequest {
    query: "What are the latest findings on neuro-symbolic AI?".to_string(),
    namespace: "research".to_string(),
    vector_k: 10,       // Top-K vectors
    graph_depth: 2,    // Expand graph 2 levels deep from results
    mode: SearchMode::Hybrid as i32,
    limit: 5,
};

let response = engine.hybrid_search(Request::new(request)).await?;
```

### 5. Automated Reasoning

Apply OWL-RL or RDFS reasoning to derive implicit knowledge:

```rust
use synapse_core::server::proto::{ReasoningRequest, ReasoningStrategy};

let request = ReasoningRequest {
    namespace: "ontology".to_string(),
    strategy: ReasoningStrategy::Owlrl as i32,
    materialize: true, // Save inferred triples to storage
};

let response = engine.apply_reasoning(Request::new(request)).await?;
println!("Inferred {} new facts", response.into_inner().triples_inferred);
```

### 6. SPARQL Queries

Query your knowledge graph using SPARQL:

```rust
use synapse_core::server::semantic_engine::SparqlRequest;

let sparql_query = r#"
    SELECT ?subject ?predicate ?object
    WHERE {
        ?subject ?predicate ?object .
    }
    LIMIT 10
"#;

let request = SparqlRequest {
    query: sparql_query.to_string(),
    namespace: "default".to_string(),
};

let response = engine.query_sparql(Request::new(request)).await?;
println!("Results: {}", response.into_inner().results_json);
```

### 7. Multi-Namespace Usage

Isolate different knowledge domains:

```rust
// Work-related knowledge
engine.ingest_triples(Request::new(IngestRequest {
    triples: work_triples,
    namespace: "work".to_string(),
})).await?;

// Personal knowledge
engine.ingest_triples(Request::new(IngestRequest {
    triples: personal_triples,
    namespace: "personal".to_string(),
})).await?;

// Query specific namespace
let work_data = engine.get_all_triples(Request::new(EmptyRequest {
    namespace: "work".to_string(),
})).await?;
```

## 📚 API Reference

### gRPC API

The `SemanticEngine` service provides the following RPC methods:

| Method                | Request               | Response            | Description                            |
| --------------------- | --------------------- | ------------------- | -------------------------------------- |
| `IngestTriples`       | `IngestRequest`       | `IngestResponse`    | Add RDF triples to the graph           |
| `GetNeighbors`        | `NodeRequest`         | `NeighborResponse`  | Graph traversal (supports edge & type filters) |
| `Search`              | `SearchRequest`       | `SearchResponse`    | Legacy vector search                   |
| `ResolveId`           | `ResolveRequest`      | `ResolveResponse`   | Resolve URI string to internal node ID |
| `GetAllTriples`       | `EmptyRequest`        | `TriplesResponse`   | Retrieve all triples from a namespace  |
| `QuerySparql`         | `SparqlRequest`       | `SparqlResponse`    | Execute SPARQL 1.1 queries             |
| `DeleteNamespaceData` | `EmptyRequest`        | `DeleteResponse`    | Delete all data in a namespace         |
| `HybridSearch`        | `HybridSearchRequest` | `SearchResponse`    | AI Search (Vector + Graph)             |
| `ApplyReasoning`      | `ReasoningRequest`    | `ReasoningResponse` | Trigger deductive inference            |

**Proto Definition**: See [`semantic_engine.proto`](https://github.com/pmaojo/synapse-engine/blob/main/crates/semantic-engine/proto/semantic_engine.proto)

### MCP Tools

When running in `--mcp` mode, the engine exposes a rich set of tools via `tools/list` and `tools/call`.
All tool inputs are strictly validated against their JSON Schema definitions.

#### `query_graph`

Retrieve all triples from a namespace.

**Input Schema:**

```json
{
  "namespace": "string (default: robin_os)"
}
```

#### `ingest_triple`

Add a new RDF triple to the knowledge graph.

**Input Schema:**

```json
{
  "subject": "string (required)",
  "predicate": "string (required)",
  "object": "string (required)",
  "namespace": "string (default: robin_os)"
}
```

#### `query_sparql`

Execute a SPARQL query on the knowledge graph.

**Input Schema:**

```json
{
  "query": "string (required)",
  "namespace": "string (default: robin_os)"
}
```

### Security & Authorization

Synapse implements a token-based authorization system. When using gRPC, tokens are extracted from the `Authorization: Bearer <token>` header.
Permissions are defined via the `SYNAPSE_AUTH_TOKENS` environment variable (JSON format).

Supported permissions:
- `read`: Query data (`GetNeighbors`, `Search`, `SparqlQuery`, etc.)
- `write`: Ingest data (`IngestTriples`, `IngestFile`)
- `delete`: Delete data (`DeleteNamespaceData`)
- `reason`: Trigger reasoning (`ApplyReasoning`)

## 🏗️ Architecture

### Storage Layer

- **Oxigraph**: RDF triple store with SPARQL 1.1 support
- **Namespace Isolation**: Each namespace gets its own persistent storage directory
- **URI Mapping**: Automatic conversion between URIs and internal node IDs for gRPC compatibility

### Reasoning Engine

- **Reasonable**: OWL RL reasoning for automatic inference
- **Deductive Capabilities**: Derive new facts from existing triples using ontological rules

### Dual-Mode Operation

```
┌─────────────────────────────────────┐
│      Synapse Core Engine            │
├─────────────────────────────────────┤
│                                     │
│  ┌──────────────┐  ┌─────────────┐ │
│  │  gRPC Server │  │  MCP Server │ │
│  │  (Port 50051)│  │  (stdio)    │ │
│  └──────┬───────┘  └──────┬──────┘ │
│         │                 │         │
│         └────────┬────────┘         │
│                  │                  │
│         ┌────────▼────────┐         │
│         │ MySemanticEngine│         │
│         └────────┬────────┘         │
│                  │                  │
│         ┌────────▼────────┐         │
│         │  SynapseStore   │         │
│         │  (per namespace)│         │
│         └────────┬────────┘         │
│                  │                  │
│         ┌────────▼────────┐         │
│         │   Oxigraph RDF  │         │
│         │   Triple Store  │         │
│         └─────────────────┘         │
└─────────────────────────────────────┘
```

### Namespace Management

Each namespace is completely isolated with its own:

- Storage directory (`{GRAPH_STORAGE_PATH}/{namespace}`)
- Oxigraph store instance
- URI-to-ID mapping tables

This enables multi-tenant scenarios and context separation.

## ⚙️ Configuration

### Environment Variables

| Variable                | Default       | Description                                  |
| ----------------------- | ------------- | -------------------------------------------- |
| `GRAPH_STORAGE_PATH`    | `data/graphs` | Root directory for namespace storage         |
| `HUGGINGFACE_API_TOKEN` | `(optional)`  | Token for Inference API (higher rate limits) |

### Storage Structure

```
data/graphs/
├── default/          # Default namespace
├── work/             # Work namespace
└── personal/         # Personal namespace
```

## 🤝 Contributing

Contributions are welcome! Please check the [repository](https://github.com/pmaojo/synapse-engine) for guidelines.

## 📄 License

This project is licensed under the [MIT License](LICENSE).

---

**Built with ❤️ using Rust, Oxigraph, and Tonic**
