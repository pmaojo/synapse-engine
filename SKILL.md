# Synapse Skill 🧠⛓️

Memory layer for OpenClaw agents powered by a Rust-based neuro-symbolic knowledge graph.

## Configuration

Synapse runs as an MCP server. OpenClaw auto-registers it if this directory is in your skills path.

To enable Synapse as your primary memory provider, update your `openclaw.json`:

```json
{
  "agents": {
    "defaults": {
      "memorySearch": {
        "provider": "mcp",
        "mcpServer": "synapse"
      }
    }
  }
}
```

## Tools

- `query_graph`: Search semantic triples.
- `ingest_triple`: Add new knowledge (Subject, Predicate, Object).
