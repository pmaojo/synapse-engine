# Synapse API Documentation

## 🌐 Gradio API

Synapse exposes a **REST API** via Gradio's built-in API functionality.

### Base URL
```
http://localhost:7860
```

### API Endpoints

#### 1. **Chat Assistant** - `/api/predict`
Send natural language queries to the autonomous assistant.

**Request:**
```bash
curl -X POST http://localhost:7860/api/predict \
  -H "Content-Type: application/json" \
  -d '{
    "data": ["What plants improve soil?"]
  }'
```

**Response:**
```json
{
  "data": [
    "🤖 Autonomous Orchestrator\n\nTools Used:\n- nl2cypher: Query keywords detected\n\n🔍 Query: MATCH (x)-[:improves]->(s) WHERE s CONTAINS 'Soil' RETURN x\n\nFound 3 results:\n1. Compost --[improves]--> Soil\n2. Mulch --[improves]--> Soil\n3. GreenManure --[improves]--> Soil"
  ]
}
```

#### 2. **Extract Triples** - `/api/extract_triples`
Extract semantic triples from text.

**Request:**
```bash
curl -X POST http://localhost:7860/api/extract_triples \
  -H "Content-Type: application/json" \
  -d '{
    "data": ["Compost improves soil fertility"]
  }'
```

**Response:**
```json
{
  "data": [
    "Extracted 1 triples:\n(Compost, improves, Soil)"
  ]
}
```

#### 3. **Run Pipeline** - `/api/run_pipeline`
Execute data processing pipelines.

**Request:**
```bash
curl -X POST http://localhost:7860/api/run_pipeline \
  -H "Content-Type: application/json" \
  -d '{
    "data": ["DataSyn Processor", "documents/DataSyn/plants.csv"]
  }'
```

**Response:**
```json
{
  "data": [
    "✅ Complete: 2,412 triples extracted"
  ]
}
```

---

## 🔌 gRPC API (Rust Backend)

The Rust semantic engine exposes a gRPC API for high-performance graph operations.

### Proto Definition
```protobuf
service SemanticEngine {
  rpc IngestTriples(IngestRequest) returns (IngestResponse);
  rpc QueryNeighbors(QueryRequest) returns (QueryResponse);
  rpc GetAllTriples(EmptyRequest) returns (TriplesResponse);
}
```

### Python Client Usage

```python
from agents.grpc.client import get_client

# Connect to Rust backend
client = get_client()

# Ingest triples
result = client.ingest_triples([
    ("Apple", "belongsTo", "Rosaceae"),
    ("Pear", "belongsTo", "Rosaceae")
])
print(f"Added {result['edges_added']} edges")

# Get all triples
triples = client.get_all_triples()
print(f"Total triples: {len(triples)}")
```

---

## 🧩 SPARQL Endpoint

**Status:** ✅ **Implemented**

Execute SPARQL queries against the RDF ontology graph.

### Python API

```python
from agents.retrieval.sparql_engine import SPARQLEngine
from agents.core.ontology import OntologyService

# Load ontology
ontology = OntologyService(["ontology/core.owl", "ontology/agriculture.owl"])

# Create SPARQL engine
sparql = SPARQLEngine(ontology.graph)

# Execute query
query = """
SELECT ?subject ?predicate ?object WHERE {
    ?subject ?predicate ?object .
    FILTER(CONTAINS(STR(?object), "Soil"))
} LIMIT 10
"""

results = sparql.query(query)
for row in results:
    print(f"{row['subject']} - {row['predicate']} - {row['object']}")
```

### Available SPARQL Methods

```python
# Get all classes
classes = sparql.get_all_classes()

# Get all properties
properties = sparql.get_all_properties()

# Get instances of a class
instances = sparql.get_instances("Plant")

# Custom SPARQL query
results = sparql.query("SELECT * WHERE { ?s ?p ?o } LIMIT 100")
```

---

## 🤖 MCP (Model Context Protocol) API

Synapse implements MCP for LLM tool integration.

### Available Tools

1. **query_knowledge_graph** - Query via RAG or SPARQL
2. **add_observation** - Add new triples from text
3. **sparql_query** - Execute SPARQL directly
4. **get_ontology_schema** - Retrieve ontology structure

### MCP Server Usage

```python
from agents.server.mcp_server import MCPServer

server = MCPServer(["ontology/core.owl"])

# Query knowledge graph
result = await server.query_knowledge_graph({
    "query": "What improves soil?",
    "top_k": 5
})

# Execute SPARQL
result = await server.sparql_query({
    "query": "SELECT ?s WHERE { ?s a :Plant } LIMIT 10"
})
```

---

## 📊 Python SDK

### Core Components

#### 1. **Pipeline API**
```python
from agents.pipelines.datasyn import DataSynPipeline

pipeline = DataSynPipeline()
result = pipeline.run("data/plants.csv")

print(f"Success: {result.success}")
print(f"Triples: {result.data['triples_extracted']}")
print(f"Time: {result.execution_time:.2f}s")
```

#### 2. **NL2Cypher API**
```python
from agents.tools.nl2cypher import NL2CypherAgent

agent = NL2CypherAgent()
cypher = await agent.translate("What is in Rosaceae?")
print(cypher)
# Output: MATCH (x)-[:belongsTo]->(f) WHERE f = 'Rosaceae' RETURN x
```

#### 3. **OWL Reasoning API**
```python
from agents.tools.owl_reasoner import OWLReasoningAgent
from agents.core.ontology import OntologyService

ontology = OntologyService(["ontology/core.owl"])
reasoner = OWLReasoningAgent(ontology.graph)

result = reasoner.infer([
    ("Compost", "improves", "Soil"),
    ("Soil", "supports", "Plants")
])

print(f"Inferred {len(result['inferred_triples'])} new triples")
print(f"Expansion: {result['expansion_ratio']:.1%}")
```

#### 4. **Ontology Validation API**
```python
from agents.validation.ontology_validator import OntologyValidator

validator = OntologyValidator(ontology)

result = validator.validate_triple("Plant", "hasProperty", "Color")
if result["valid"]:
    print("✅ Triple is valid")
else:
    print(f"❌ Errors: {result['errors']}")
    print(f"Suggestions: {result['suggestions']}")
```

---

## 🔧 Configuration

### Environment Variables

```bash
# .env file
GEMINI_API_KEY=your_api_key_here
GEMINI_MODEL=gemini-2.5-flash
RUST_BACKEND_HOST=localhost
RUST_BACKEND_PORT=50051
```

### MCP Configuration

```json
// .mcp/config.json
{
  "mcpServers": {
    "synapse-semantic-engine": {
      "command": "../../target/release/semantic-engine"
    },
    "synapse-python-tools": {
      "command": "python",
      "args": ["-m", "agents.server.mcp_server"]
    }
  },
  "orchestration": {
    "enabled": true,
    "autonomous_mode": true,
    "allow_tool_composition": true
  }
}
```

---

## 📝 Response Formats

### Pipeline Result
```python
{
    "success": True,
    "data": {
        "file": "plants.csv",
        "triples_extracted": 2412,
        "triples": ["(Apple, belongsTo, Rosaceae)", ...],
        "total_chunks_processed": 10
    },
    "logs": ["📄 Processing: plants.csv", ...],
    "execution_time": 12.5
}
```

### OWL Reasoning Result
```python
{
    "inferred_triples": [
        ("Compost", "rdf:type", "Material"),
        ("Soil", "rdf:type", "NaturalResource")
    ],
    "rules_applied": {
        "rdfs:subClassOf": 5,
        "rdfs:domain": 7,
        "owl:transitiveProperty": 3
    },
    "expansion_ratio": 2.5,
    "original_count": 10,
    "inferred_count": 15,
    "total_count": 25
}
```

---

## 🧪 Testing

Run E2E tests:
```bash
pytest tests/test_e2e.py -v
```

Test specific functionality:
```bash
# Test NL2Cypher
pytest tests/test_e2e.py::test_nl2cypher -v

# Test OWL reasoning
pytest tests/test_e2e.py::test_owl_reasoning -v

# Test complete workflow
pytest tests/test_e2e.py::test_e2e_workflow -v
```

---

## 📚 Additional Resources

- [Architecture Overview](docs/architecture.md)
- [Pipeline Guide](docs/pipelines.md)
- [Ontology Design](docs/ontology.md)
- [SOLID Analysis](docs/solid_analysis.md)
