# Synapse: The AGI Memory Core
**Hypothesis: The True Power Lies in the Symbolic Engine, Not Vector Embeddings**

*An exploration of `synapse-core` as the foundational memory architecture for Autonomous Agents.*

---

## 1. The Core Hypothesis: Can We Drop Embeddings entirely?

Current AI trends aggressively push for Vector/RAG architectures, relying heavily on embeddings (e.g., `fastembed`, HNSW) to recall information. However, this approach comes with significant overhead (computation, storage, dimensionality curses) and a fundamental flaw: **lack of logical precision.**

**The Hypothesis:** We could completely discard embeddings and rely exclusively on a pure symbolic knowledge graph.

**Why this works (Codebase Reality):**
Synapse's true differentiator is already built into the `synapse-core` layer—powered by Oxigraph for sub-millisecond RDF triple operations, combined with the built-in OWL-RL and RDFS reasoner (`SynapseReasoner`).
*   Embeddings give us *fuzzy similarity* ("This sounds like X").
*   The Graph gives us *absolute truth and derivation* ("X *is a* Y, and Y *implies* Z").

If agents interact through well-defined ontologies (like the ones currently in `scenarios/core/schema`), exact entity matching via SPARQL and graph traversal (BFS expansion in `server.rs`) can yield more accurate, deterministic context for LLMs than vector similarity ever could.

---

## 2. The Core Benefit: `synapse-core` is All You Need

The market is flooded with vector databases that are essentially black boxes. The real need for AGI is **explainable, deterministic memory.**

The core benefit of this project is the `synapse-core` Rust crate itself, completely stripped of external dependencies or bloated machine learning components.

**Market Need:** Agents need a persistent state machine that understands rules.
*   **Speed:** Running purely in Rust/Oxigraph allows for tens of thousands of logical inferences per second.
*   **Portability:** By dropping `onnxruntime` and embedding models, the engine becomes infinitesimally small. It can run anywhere—edge devices, embedded systems, or within a simple WASM container.
*   **Reasoning:** The ability to derive *implicit* knowledge (transitivity, symmetry) from explicit facts is the hallmark of human-like memory.

---

## 3. The Medium: Semantic Graphs vs. Markdown Files

A common pattern for developers building agent memory is simply dumping conversations and facts into raw Markdown (`.md`) files and hoping the LLM's context window can sort it out.

**Why Graphs are Better:**
*   MD files are unstructured. Querying them requires expensive LLM processing (RAG) every single time.
*   Graphs (`graph.nq` / RocksDB) are strictly typed. We know exactly *who* did *what* and *when* without re-reading a 500-page document.

**The Golden Combination (The Hybrid Hypothesis):**
The ideal architecture is a symbiotic relationship between raw text and semantic indices.
*   **Storage:** Keep source documents (conversations, papers, logs) as simple Markdown files for human readability and base ground-truth.
*   **Index:** Synapse acts as the *semantic index* overlaying those files. When a markdown file is saved, Synapse parses the *entities* and *relationships* and stores them as RDF triples, pointing back to the MD file via Provenance metadata (e.g., `prov:wasDerivedFrom`).

When the AGI needs to remember something, it queries the lightweight graph for the exact location and logical context, then pulls only the specific chunk of the MD file needed.

---

## 4. Next Steps: Evolving into the AGI Engine

To prove these hypotheses and solidify Synapse as the premier AGI memory core, we must execute the following:

1.  **Decouple and Feature-Flag Vectors (Aggressively):** While `local-embeddings` is currently a feature flag, we should actively benchmark and promote the "Light Mode" (Graph-only) as the default, preferred path for agentic logic.
2.  **Enhance the Reasoner:** Expand the OWL-RL implementation to support more complex logic (e.g., disjoint classes, property chains) to make the engine "smarter" without needing an LLM.
3.  **Graph-to-Markdown Sync:** Build a native pipeline that reads a folder of `.md` files, parses frontmatter and structured text into triples, and uses Synapse solely to map the semantic relationships between the files.
4.  **Agent Protocol (MCP) First:** Double down on the Model Context Protocol. The AGI doesn't need a UI; it needs a perfectly structured, low-latency API to read and write its own synaptic connections.

*Synapse is not just another vector database. It is the logic board for the next generation of artificial minds.*