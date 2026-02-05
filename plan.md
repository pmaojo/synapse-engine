# Synapse: The Memory Layer for Agentic OS

Synapse is being evolved from a prototype into the foundational memory layer for **Robin OS**. This plan focuses on performance, integration, and agentic autonomy.

## 1. Vision
Synapse is not just a database; it is a **Neuro-symbolic Brain**. It provides:
- **Formal Memory**: Strict relationships via Ontologies (OWL).
- **Episodic Memory**: Vectorized search over past interactions.
- **Relational Memory**: Fast graph traversal via Rust.

## 2. Core Components
- **Synapse-Core (Rust)**: High-performance graph engine.
- **Synapse-MCP**: Native Model Context Protocol server for LLM integration.
- **Synapse-Orchestrator (Python)**: Intelligent extraction and translation layer.

## 3. Roadmap (Robin OS Era)

### Phase 1: Integration & Portability ✅
- [x] Port core logic to Rust for 100x performance.
- [x] Implement gRPC interface for multi-language support.
- [x] **New**: Native MCP (stdio) support for direct LLM usage.
- [x] Rebrand from Grafoso to Synapse.
- [x] **Multi-Tenant Architecture**: Dynamic namespace support for context separation (Work, Personal, OS).

### Phase 2: Knowledge Ingestion 🏗️
- [ ] **Smart Extractor**: LLM-driven pipeline to turn documentation into triples.
- [ ] **Ontology Expansion**: Define formal schemas for Frontend Engineering and Project Management.
- [ ] **Data Migration**: Move existing "Cerebro" (SQLite) data to Synapse.

### Phase 3: Agentic Autonomy 🦜
- [ ] **Self-Reflection**: Periodic tasks where Robin reviews memory and compacts/optimizes the graph.
- [ ] **Cross-Agent Memory**: Enable different sub-agents to share knowledge via Synapse.
- [ ] **Notion Sync**: Continuous backup of critical graph nodes to Notion.

## 4. Development Guidelines
- **Rust First**: Performance-critical operations must live in `crates/`.
- **Contract Driven**: Use Protobuf and JSON-RPC (MCP) for all communications.
- **Privacy**: Local-first by default. External APIs (Gemini/OpenAI) are only for extraction logic, not storage.

---
*Robin OS - Productivity & Professional Innovation*
