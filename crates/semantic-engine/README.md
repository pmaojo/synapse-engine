# Synapse Core 🧠

<div align="center">

[![Crates.io](https://img.shields.io/crates/v/synapse-core.svg)](https://crates.io/crates/synapse-core)
[![Documentation](https://docs.rs/synapse-core/badge.svg)](https://docs.rs/synapse-core)
[![License](https://img.shields.io/crates/l/synapse-core.svg)](https://github.com/pmaojo/synapse-engine/blob/main/LICENSE)

**A high-performance neuro-symbolic semantic engine designed for agentic AI.**

[Features](#-features) • [Installation](#-installation) • [Usage](#-usage) • [Architecture](#-architecture)

</div>

---

## 📖 Overview

**Synapse Core** provides the foundational semantic memory layer for AI agents. It combines the structured precision of **Knowledge Graphs** (using [Oxigraph](https://github.com/oxigraph/oxigraph)) with the flexibility of **Vector Stores**, allowing agents to reason about data, maintain long-term context, and deduce new information automatically.

It is designed to work seamlessly with **OpenClaw** and other agentic frameworks via the **Model Context Protocol (MCP)** or as a standalone **gRPC service**.

## 🚀 Features

- **Neuro-Symbolic Engine**: Hybrid architecture merging graph databases (RDF/SPARQL) with semantic vector search.
- **Reasoning Capabilities**: Built-in support for OWL reasoning via `reasonable`, enabling automatic deduction of facts.
- **MCP Support**: Native implementation of the Model Context Protocol for easy integration with LLM agents.
- **High Performance**: Written in Rust for minimal latency and maximum throughput.
- **Namespace Isolation**: Support for multiple isolated knowledge bases (e.g., "work", "personal").

## 📦 Installation

To use `synapse-core` in your Rust project:

```toml
[dependencies]
synapse-core = "0.2.0"
```

To install the binary CLI:

```bash
cargo install synapse-core
```

## 🛠️ Usage

### 1. Standalone gRPC Server

Run Synapse as a high-performance gRPC server:

```bash
# Start the server (default port 50051)
synapse
```

Configuration via environment variables:

- `GRAPH_STORAGE_PATH`: Directory to persist graph data (default: `data/graphs`).

### 2. Model Context Protocol (MCP)

Synapse is designed to plug directly into agents like **OpenClaw** or IDEs like **Cursor** via MCP (stdio mode).

```bash
# Start in MCP mode
synapse --mcp
```

### 3. Rust Library

Embed the engine directly into your application:

```rust
use synapse_core::server::MySemanticEngine;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let engine = MySemanticEngine::new("data/my_graph");

    // Use the engine instance...
}
```

## 🏗️ Architecture

Synapse is built on a robust stack:

- **Storage**: [Oxigraph](https://github.com/oxigraph/oxigraph) for RDF triple storage and SPARQL querying.
- **Reasoning**: [Reasonable](https://github.com/gtfierro/reasonable) for OWL RL reasoning.
- **Transport**: [Tonic](https://github.com/hyperium/tonic) for gRPC and [Tokio](https://tokio.rs) for async runtime.

## 🤝 Contributing

Contributions are welcome! Please check the [repository](https://github.com/pmaojo/synapse-engine) for guidelines.

## 📄 License

This project is licensed under the [MIT License](LICENSE).
