# Synapse 🧠⛓️

**Synapse** is a high-performance, neuro-symbolic knowledge graph system designed to serve as the long-term memory for agentic AI. 

Built with **Rust** for the core semantic engine and **Python** for the intelligent orchestration layer, Synapse combines the formal rigor of ontologies (OWL/RDF) with the flexibility of vector embeddings.

## 🚀 Features

- **Blazing Fast Core**: Rust-based graph engine with gRPC and native MCP support.
- **Neuro-Symbolic Architecture**: Combines symbolic reasoning (triples, ontologies) with neural search (vector embeddings).
- **Native MCP Support**: Can be plugged directly into LLM environments (like Cursor or OpenClaw) as a Model Context Protocol server.
- **Ontology Driven**: Support for OWL files to define strict domain boundaries and relationships.
- **Tenant Isolation**: Multi-tenant architecture for managing different knowledge bases.

## 🏗️ Architecture

- **`crates/semantic-engine`**: The Rust heartbeat. Handles persistence, graph topology, and gRPC/MCP interfaces.
- **`agents/`**: Python orchestration layer. Handles intelligent extraction, translation, and high-level pipelines.
- **`ontology/`**: Formal definitions of the world.

## 🛠️ Quick Start

### Prerequisites
- Rust (latest stable)
- Python 3.10+
- Protobuf Compiler (`protoc`)

### Running as a gRPC Server
```bash
./start_rust_server.sh
```

### Running as an MCP Server (stdio)
```bash
./start_rust_server.sh --mcp
```

## 🦜 Identity
Synapse is the foundational memory of **Robin OS**, the personal productivity agent for Frontend Leads and Creative Engineers.

---
*Developed by the Synapse Team (ex-Synapse)*
