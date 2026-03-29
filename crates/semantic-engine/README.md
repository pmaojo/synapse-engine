# Synapse Core 🧠

<div align="center">

[![Crates.io](https://img.shields.io/crates/v/synapse-core.svg)](https://crates.io/crates/synapse-core)
[![Documentation](https://docs.rs/synapse-core/badge.svg)](https://docs.rs/synapse-core)
[![License](https://img.shields.io/crates/l/synapse-core.svg)](https://github.com/pmaojo/synapse-engine/blob/main/LICENSE)

**The Symbolic AGI Memory Core (Pure Graph, No Vectors)**

[Features](#-features) • [Installation](#-installation) • [Architecture](#-architecture) • [MCP Ext-Apps UI](#-mcp-ext-apps-ui)

</div>

---

## 📖 Overview

**Synapse Core** provides the foundational semantic memory layer for Autonomous AI agents.

Following a radical architectural pivot, Synapse Core has completely **eradicated vector embeddings and probabilistic machine learning**. The true power of an AGI's memory lies in its symbolic engine: deterministic, logical, and fully explainable.

Powered by [Oxigraph](https://github.com/oxigraph/oxigraph) and an optimized OWL-RL Reasoner in pure Rust, it provides:
1. **Absolute Logical Truths** instead of fuzzy RAG similarity.
2. **Bidirectional Markdown Synchronization**, making the semantic graph the invisible nervous system bridging raw text files and agentic memory.
3. **Model Context Protocol (MCP)** exclusively, built for the next generation of LLMs.

## 🚀 Features

- **Pure Symbolic Graph**: 100% deterministic memory. No `onnxruntime`, no RAG vector stores.
- **Bidirectional Markdown Sync (MD <-> Graph)**:
  - **Read**: Extracts entities, relationships (`[[wikilinks]]`), and `YAML Frontmatter`. Hashes blocks for precise `prov:wasDerivedFrom` provenance.
  - **Write**: The reasoner automatically injects inferred logical truths back into the Markdown files as "Backlinks" and updates the Frontmatter without destroying human-readable text.
- **Fixed-Point OWL-RL Reasoner**: Calculates the transitive closure of the graph (Symmetry, Transitivity, Property Chains) looping in-memory until no new implicit knowledge can be derived.
- **MCP Ext-Apps UI Support**: Exposes interactive HTML/JS graph visualizations (e.g., node neighborhoods via D3) using the `ui://` protocol for clients like Claude Desktop.
- **Dual Transport MCP Server**:
  - STDIO transport for local agents (`--stdio`)
  - HTTP/SSE (Server-Sent Events) for distributed Ext-Apps.

## 📦 Installation

Install the CLI tool directly from source:

```bash
cargo install --path crates/semantic-engine
```

## 🛠️ Integration Guide for Agents (MCP)

Synapse is designed to be invisible to humans but fully controllable by Autonomous Agents via the **Model Context Protocol (MCP)**.

### 1. Starting the Server

**Local Agent Mode (STDIO):**
Used by local CLI agents or Claude Desktop.
```bash
synapse --stdio
```

**Ext-Apps Mode (HTTP/SSE):**
Used to expose the engine to web interfaces or network-distributed agents.
```bash
SYNAPSE_MCP_PORT=3000 synapse
```

### 2. Available MCP Tools

Agents can discover and use the following tools:

- `sparql_query`: Execute raw SPARQL 1.1 queries to traverse complex logical paths in the memory.
- `get_entity_neighborhood`: Given an entity URI, returns the deterministic BFS expansion of its subgraph. It also returns an embedded `ui://` resource for the host to render a visual graph!
- `index_markdown_directory`: Commands the engine to crawl a folder, parse all `.md` files, extract their semantic links, and persist them into the graph.

### 3. Using the Markdown Sync

Instead of black-box RAG, agents dump their knowledge into human-readable `.md` files.

1. Agent creates `docs/concept.md` containing: `This is related to [[AnotherConcept]]`.
2. Agent calls `index_markdown_directory` with `docs/`.
3. Synapse parses the file, creates RDF triples linking `concept` -> `AnotherConcept`, and tracks provenance.
4. If `AnotherConcept` implies a new rule via the Reasoner, Synapse **writes back** to the `.md` file, appending a `## 🧠 Synapse Backlinks` section automatically.

## 🏗️ Architecture

```
┌───────────────────────────────────────────┐
│              Agent (LLM)                  │
└──────────────────┬────────────────────────┘
                   │ MCP (JSON-RPC)
┌──────────────────▼────────────────────────┐
│             Synapse Core                  │
│                                           │
│  ┌────────────────┐    ┌───────────────┐  │
│  │   MCP Server   │────│ MD Sync Engine│  │
│  │ (Stdio / SSE)  │    │(Parser/Writer)│  │
│  └───────┬────────┘    └───────┬───────┘  │
│          │                     │          │
│  ┌───────▼─────────────────────▼───────┐  │
│  │        SynapseReasoner (OWL-RL)     │  │
│  │        (Fixed-Point Iteration)      │  │
│  └───────┬─────────────────────────────┘  │
│          │                                │
│  ┌───────▼─────────────────────────────┐  │
│  │  Oxigraph Store (RocksDB / Memory)  │  │
│  └─────────────────────────────────────┘  │
└───────────────────────────────────────────┘
```

## ⚙️ Configuration

| Variable                | Default       | Description                                  |
| ----------------------- | ------------- | -------------------------------------------- |
| `GRAPH_STORAGE_PATH`    | `data/graphs` | Root directory for RocksDB storage           |
| `SYNAPSE_MCP_PORT`      | `3000`        | Port for the HTTP/SSE Ext-Apps Server        |

## 📄 License

This project is licensed under the [MIT License](../../LICENSE).
