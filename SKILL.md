---
name: synapse
description: "Synapse: High-performance neuro-symbolic knowledge graph for OpenClaw. Provides structured long-term memory, reasoning, and hybrid search."
license: MIT
metadata:
  version: "0.6.0"
  author: pmaojo
---

# SKILL: synapse

Synapse es el motor semántico neuro-simbólico de Robin (OpenClaw). Proporciona memoria a largo plazo estructurada, razonamiento y búsqueda híbrida.

## 🛠️ Herramientas

### 1. Python SDK (synapse-sdk)
He empaquetado la lógica de integración como un SDK instalable. Puedes usarlo en cualquier script:

```python
from synapse import get_client

client = get_client()
client.ingest_triples([
    {"subject": "Pelayo", "predicate": "is", "object": "Expert"}
], namespace="work")
```

### 2. Ingestión de Conocimiento
- **Notion Sync**: Trae notas recientes de Notion y las convierte en RDF.
  ```bash
  python3 scripts/ingest_notion.py
  ```

### 3. Razonamiento (Reasoning)
Ejecuta el razonador OWL-RL para inferir nuevos hechos basados en ontologías.
- **Script**: `scripts/reason.py`
- **Uso**:
  ```bash
  python3 scripts/reason.py --namespace <ns> --strategy OWLRL
  ```

### 4. Consultas (SPARQL)
Realiza consultas complejas al grafo.
- **Script**: `scripts/sparql.py`
- **Uso**:
  ```bash
  python3 scripts/sparql.py "SELECT ?s ?p ?o WHERE { ?s ?p ?o } LIMIT 10"
  ```

## 🧠 Ontologías Soportadas
Synapse soporta ontologías estándar. Prefijos comunes pre-cargados:
- `rdf`: http://www.w3.org/1999/02/22-rdf-syntax-ns#
- `rdfs`: http://www.w3.org/2000/01/rdf-schema#
- `owl`: http://www.w3.org/2002/07/owl#
- `schema`: http://schema.org/
- `dc`: http://purl.org/dc/terms/ (Dublin Core)
- `skos`: http://www.w3.org/2004/02/skos/core#

## 🔄 Flujo de Trabajo
1. **Ingestar**: Traer datos crudos (Notion, Logs, etc.).
2. **Razonar**: Ejecutar `reason.py` para materializar inferencias (ej: Si `A es tipo Perro`, inferir `A es tipo Animal`).
3. **Consultar**: Usar SPARQL para recuperar respuestas complejas.
