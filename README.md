# Synapse 🧠⛓️

**Synapse** is a high-performance, neuro-symbolic knowledge graph system designed to serve as the long-term memory for agentic AI. 

## 🚀 One-Click Install for OpenClaw

Just run:
```bash
npx skills install pmaojo/synapse-engine
```

OpenClaw will automatically detect Synapse as an MCP server.

## 🛠️ Configuration

To use Synapse as your agent's memory, add this to your `openclaw.json`:

```json
"memorySearch": {
  "provider": "mcp",
  "mcpServer": "synapse"
}
```

## 🌐 Notion Sync: Automated Omnipresence

Synapse can automatically convert your Notion notes into structured knowledge.

### 1. Setup Notion Integration
Make sure your OpenClaw environment has the Notion skill configured with your `NOTION_API_KEY`.

### 2. Enable Auto-Sync
Add a sync job to your `openclaw.json` or use the built-in cron:
```bash
openclaw cron add --name "Notion to Synapse" --every "1h" --message "Read my recent Notion pages and ingest them into Synapse namespace 'personal'"
```

### 3. How it works
Robin will:
1. Fetch new content from your linked Notion databases.
2. Extract semantic triples using LLM reasoning.
3. Ingest them into the specified Synapse namespace.
4. Your notes are now queryable via SPARQL or natural language!

## 🏗️ Technical Architecture

### 1. Ontology Management
Ontologies are defined in standard OWL format within the `ontology/` directory (e.g., `core.owl`). 
*   **Classes**: Hierarchical definitions (e.g., `Process` subClassOf `Event`).
*   **Properties**: Object and Datatype properties with defined `domain` and `range`.
*   **Evolution**: Following the **Architect's Loop**, new classes and properties are proposed when data doesn't fit existing schemas.

### 2. Triple Validation
Ingested triples are validated against current ontologies:
*   **Semantic Consistency**: Ensures that subjects and objects match property domain/range constraints.
*   **Inference-based Validation**: Uses the reasoner to check if new facts contradict existing knowledge (e.g., disjoint class violations).

### 3. Reasoning Engine
The **Synapse Reasoner** is implemented in Rust, providing:
*   **RDFS Strategy**: Implements class and property transitivity.
*   **OWL-RL Strategy**: Advanced rules including `SymmetricProperty`, `TransitiveProperty`, and `inverseOf` logic.
*   **Materialization**: Derives implicit triples and stores them in the graph for fast retrieval.

## 🏗️ Features

- **Blazing Fast Core**: Rust-based graph engine (Oxigraph).
- **Native MCP**: Plugs directly into OpenClaw/Cursor.
- **Reasoning Engine**: Built-in OWL reasoning.
- **Namespace Isolation**: Manage multiple knowledge bases (Work, Personal, Research).

---
*Developed by the Synapse Team*
