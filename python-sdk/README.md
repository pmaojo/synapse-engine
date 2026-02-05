# Synapse Python SDK 🐍🦀

Official Python interface for the **Synapse Semantic Engine**.

## Installation

```bash
pip install .
```

## Quick Start

```python
from synapse import get_client

# Connect to the local Synapse motor
client = get_client()

# Ingest knowledge
client.ingest_triples([
    {
        "subject": "Pelayo",
        "predicate": "expertIn",
        "object": "Neuro-symbolic AI"
    }
], namespace="work")

# Search memory
results = client.hybrid_search("What does Pelayo know about AI?", namespace="work")
print(results)
```

## Features
- **gRPC Client**: Low-latency communication with the Rust core.
- **DDD Structure**: Organized into domain, application, and infrastructure layers.
- **Multi-tenancy**: First-class support for namespaces.
