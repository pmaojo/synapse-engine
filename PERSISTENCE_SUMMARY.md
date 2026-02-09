## ✅ Solutions Implemented

### 1. **Automated Batch Processing**
Created `scripts/batch_ingest.py` - automatically processes ALL files in `documents/DataSyn/`:

```bash
.venv/bin/python scripts/batch_ingest.py
```

**What it does:**
- Finds all `.csv`, `.md`, `.json` files
- Processes them line-by-line (header included for context)
- Stores triples in Rust backend
- Shows progress and total count

### 2. **Rust Persistence Strategy**

**Problem:** If Rust server crashes, data is lost ❌

**Solution:** Hybrid approach with **auto-save**:

```
┌─────────────────────────────────────┐
│  Rust Server (In-Memory CSR)       │
│  - Fast graph operations            │
│  - Auto-save every N triples        │
│  - Graceful shutdown handler        │
└──────────────┬──────────────────────┘
               │ Periodic Save
               ▼
┌─────────────────────────────────────┐
│  uri_mappings.bin, vectors.bin      │
│  - Binary serialization (bincode)   │
│  - Atomic writes (rename)           │
│  - Loads on startup                 │
│  - Survives crashes (last save)     │
└─────────────────────────────────────┘
```

**Features:**
- ✅ **Auto-save every 100 vector items** (configurable via `VectorStore::auto_save_threshold`)
- ✅ **Graceful shutdown** (Ctrl+C triggers full flush before exit)
- ✅ **Binary Persistence** (Fast loading/saving using `bincode`)
- ✅ **Atomic Writes** (Prevents file corruption by writing to `.tmp` then renaming)
- ⚠️ **Crash recovery**: Loses only data since last auto-save (max 100 items)

**Files Created/Modified:**
- `crates/semantic-engine/src/persistence.rs` - Generic binary serialization logic
- `crates/semantic-engine/src/vector_store.rs` - Added persistence and auto-save logic
- `crates/semantic-engine/src/store.rs` - Added persistence for ID mappings
- `crates/semantic-engine/src/main.rs` - Added graceful shutdown handler

**Usage:**
The server now automatically persists data to `data/graphs/<namespace>/`.
- `uri_mappings.bin`: Maps string URIs to integer IDs.
- `vectors.bin`: Stores vector embeddings and metadata.
- Graph data is stored by Oxigraph in the same directory.

**Next Step:** Rebuild Rust server to enable persistence:
```bash
cd crates/semantic-engine
cargo build --release
```
