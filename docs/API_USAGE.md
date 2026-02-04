# Synapse API Documentation

## Overview
Synapse provides a REST API via Gradio that allows programmatic access to all features.

**Base URL:** `http://localhost:7860`

## API Endpoints

All Gradio functions are accessible via the `/api/` endpoint. The API follows Gradio's standard format.

### Available Endpoints

#### 1. Extract Triples
**Endpoint:** `/api/extract_triples`
**Method:** POST
**Description:** Extract RDF triples from text

**Request:**
```json
{
  "data": ["Your text here"]
}
```

**Response:**
```json
{
  "data": ["Extracted triples and validation results"]
}
```

#### 2. Apply OWL Reasoning
**Endpoint:** `/api/apply_reasoning`
**Method:** POST
**Description:** Apply OWL inference rules to expand knowledge

**Request:**
```json
{
  "data": ["(subject, predicate, object)\n..."]
}
```

**Response:**
```json
{
  "data": ["Inferred triples with expansion ratio"]
}
```

#### 3. Execute Cypher Query
**Endpoint:** `/api/execute_cypher`
**Method:** POST
**Description:** Execute Cypher query on the knowledge graph

**Request:**
```json
{
  "data": ["MATCH (n) RETURN n LIMIT 10"]
}
```

**Response:**
```json
{
  "data": ["Query results"]
}
```

#### 4. Chat Assistant
**Endpoint:** `/api/respond`
**Method:** POST
**Description:** Chat with the knowledge base using MCP

**Request:**
```json
{
  "data": ["What plants improve soil?", []]
}
```

**Response:**
```json
{
  "data": [[[{"role": "user", "content": "..."}, {"role": "assistant", "content": "..."}]]]
}
```

#### 5. Run Pipeline
**Endpoint:** `/api/run_pipeline`
**Method:** POST
**Description:** Execute a data processing pipeline

**Request:**
```json
{
  "data": ["DataSyn Processor", "path/to/file.csv"]
}
```

**Response:**
```json
{
  "data": ["Pipeline execution results"]
}
```

## Python Client Example

```python
from gradio_client import Client

# Connect to Synapse API
client = Client("http://localhost:7860")

# Extract triples from text
result = client.predict(
    "Compost improves soil structure and provides nutrients",
    api_name="/extract_triples"
)
print(result)

# Execute Cypher query
result = client.predict(
    "MATCH (x)-[:improves]->(s) WHERE s CONTAINS 'Soil' RETURN x",
    api_name="/execute_cypher"
)
print(result)

# Chat with assistant
result = client.predict(
    "What improves soil?",
    [],  # chat history
    api_name="/respond"
)
print(result)
```

## cURL Examples (Gradio 5.x API)

Gradio 5.x uses a two-step API process:
1. POST to `/gradio_api/call/{function_name}` to get an `event_id`
2. GET from `/gradio_api/call/{function_name}/{event_id}` to retrieve results

### Extract Triples
```bash
# Step 1: Submit request and get event_id
EVENT_ID=$(curl -X POST http://localhost:7860/gradio_api/call/extract_triples \
  -H "Content-Type: application/json" \
  -d '{"data": ["Compost improves soil structure"]}' \
  | jq -r '.event_id')

# Step 2: Get results
curl -N http://localhost:7860/gradio_api/call/extract_triples/$EVENT_ID
```

### One-liner version
```bash
curl -X POST http://localhost:7860/gradio_api/call/extract_triples \
  -H "Content-Type: application/json" \
  -d '{"data": ["Compost improves soil"]}' \
  | awk -F'"' '{print $4}' \
  | read EVENT_ID; curl -N http://localhost:7860/gradio_api/call/extract_triples/$EVENT_ID
```

### Apply OWL Reasoning
```bash
EVENT_ID=$(curl -X POST http://localhost:7860/gradio_api/call/apply_reasoning \
  -H "Content-Type: application/json" \
  -d '{"data": ["(Compost, improves, Soil)"]}' \
  | jq -r '.event_id')

curl -N http://localhost:7860/gradio_api/call/apply_reasoning/$EVENT_ID
```

### Run Pipeline
```bash
EVENT_ID=$(curl -X POST http://localhost:7860/gradio_api/call/run_pipeline \
  -H "Content-Type: application/json" \
  -d '{"data": ["DataSyn Processor", "path/to/file.csv"]}' \
  | jq -r '.event_id')

curl -N http://localhost:7860/gradio_api/call/run_pipeline/$EVENT_ID
```

### Semantic Search
```bash
EVENT_ID=$(curl -X POST http://localhost:7860/gradio_api/call/semantic_search \
  -H "Content-Type: application/json" \
  -d '{"data": ["permaculture"]}' \
  | jq -r '.event_id')

curl -N http://localhost:7860/gradio_api/call/semantic_search/$EVENT_ID
```

## API Documentation UI

Visit `http://localhost:7860/docs` to see the interactive API documentation with all available endpoints and their schemas.

## Rate Limiting

Currently no rate limiting is applied. For production use, consider adding rate limiting middleware.

## Authentication

Currently no authentication is required. For production use, consider adding API key authentication.
