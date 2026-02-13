# Synapse Memory Skill – Design

## Objective
Create an OpenClaw skill that leverages Synapse MCP for continuous memory:
- **Ingest** conversation triples at the end of each session (or periodically)
- **Query** relevant context before responding to user questions
- **Recall** people, achievements, past decisions, and technical context

## Architecture

### 1. Memory Ingestion
**When**: At the end of each conversation (or after each assistant turn)
**What**: Triples describing:
- User query + assistant response
- Entities mentioned (people, projects, technologies)
- Decisions made, actions taken
- Links to files, commits, artifacts

**How**:
- Use `mcporter call synapse.ingest_triples` (or Python SDK fallback)
- Namespace: `conversation` (or user‑specific `pelayo`)
- Example triple:
  ```json
  {
    "subject": "http://synapse.os/Pelayo",
    "predicate": "http://synapse.os/asked",
    "object": "\"How to integrate Synapse MCP for continuous memory?\""
  }
  ```

### 2. Context Retrieval
**When**: Before generating a response (FASE 0 of PRI)
**What**: Relevant triples about:
- Current user (Pelayo)
- Current project/topic
- Past similar conversations
- Technical constraints, architectural decisions

**How**:
- Use `mcporter call synapse.hybrid_search` (natural‑language query)
- Or `mcporter call synapse.sparql_query` (structured query)
- Return top‑k triples as context for the LLM

### 3. Skill Implementation
**Option A – Pure MCP** (preferred):
- No new skill needed; the agent already has MCP tools via `mcporter`.
- The Protocol (FASE 0) already mandates Synapse consultation.
- The agent must be disciplined to **always** call `synapse.hybrid_search` before answering.

**Option B – Wrapper Skill**:
- Skill `synapse‑memory` with two tools:
  - `ingest_conversation(snippet: str, namespace: str)`
  - `query_context(query: str, namespace: str, limit: int)`
- Internally calls MCP tools or Python SDK.

**Option C – Automated Cron**:
- Script `ingest_daily.py` runs hourly, reads `memory/YYYY‑MM‑DD.md`, extracts triples.
- No runtime overhead for the agent.

## Recommended Approach
**Start with Option A** (pure MCP) because:
- MCP is already 100% operational.
- No additional skill development required.
- The agent can use `mcporter` directly (as proven).
- The Protocol already enforces this.

**Enhancements later**:
- Create a cron job that ingests daily memory files.
- Build a dashboard to visualize the knowledge graph.
- Develop a `development‑framework` scenario in Synapse to model skills/rules as triples.

## Next Steps
1. **Merge PR #13** (contains all scripts and updated SKILL.md).
2. **Enforce Protocol discipline** – ensure the agent always performs FASE 0 (Synapse consultation).
3. **Test end‑to‑end flow** with a real conversation:
   - User asks a question.
   - Agent queries Synapse (`hybrid_search`).
   - Agent answers.
   - Agent ingests the Q/A pair into Synapse.
4. **Monitor and refine** the triple schema for better recall.

## Technical Notes
- **Authentication**: Token `admin_token` (already configured in systemd service).
- **Vector embeddings**: `MOCK_EMBEDDINGS=true` for now; replace with real embeddings later.
- **Storage**: `/home/robin/data/graphs` (persistent across reboots).
- **Backup**: Python SDK scripts (`ingest_now.py`, `query_context.py`) remain as fallback.