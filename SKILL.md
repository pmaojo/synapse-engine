# SKILL: synapse

Synapse es el motor semántico neuro-simbólico de Robin (OpenClaw). Proporciona memoria a largo plazo estructurada, razonamiento y búsqueda híbrida.

## 🛠️ Herramientas (Scripts Python)

Usa `exec` para invocar estos scripts. Todos requieren el entorno virtual si tienen dependencias externas, pero usan `grpcio` que instalamos en el venv.

Usa: `/home/robin/workspace/skills/synapse/.venv/bin/python3 <script> ...`

### 1. Ingestión de Conocimiento
- **Notion Sync**: Trae notas recientes de Notion y las convierte en RDF.
  ```bash
  python3 scripts/ingest_notion.py
  ```

### 2. Razonamiento (Reasoning)
Ejecuta el razonador OWL-RL para inferir nuevos hechos basados en ontologías.
- **Script**: `scripts/reason.py`
- **Uso**:
  ```bash
  python3 scripts/reason.py --namespace <ns> --strategy OWLRL
  ```

### 3. Consultas (SPARQL)
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
