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

## 🏗️ Features

- **Blazing Fast Core**: Rust-based graph engine (Oxigraph).
- **Native MCP**: Plugs directly into OpenClaw/Cursor.
- **Reasoning Engine**: Built-in OWL reasoning via `reasonable`.
- **Namespace Isolation**: Manage multiple knowledge bases (Work, Personal, Research).

---
*Developed by the Synapse Team*
