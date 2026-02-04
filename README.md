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

## 🏗️ Features

- **Blazing Fast Core**: Rust-based graph engine.
- **Native MCP**: Plugs directly into OpenClaw/Cursor.
- **Neuro-Symbolic**: Combines triples with vector embeddings.
- **Namespace Isolation**: Manage multiple knowledge bases.

---
*Developed by the Synapse Team*
