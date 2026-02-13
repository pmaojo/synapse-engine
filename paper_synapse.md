# Synapse: Un Motor de Memoria Semántica Híbrida para Sistemas de Inteligencia Artificial

## Resumen

El presente documento describe la arquitectura y funcionamiento de **Synapse**, un sistema diseñado para dotar a los Modelos de Lenguaje Grande (LLMs) de una memoria a largo plazo persistente, razonada y verificable. A diferencia de los sistemas tradicionales de Generación Aumentada por Recuperación (RAG) que dependen únicamente de la similitud vectorial, Synapse implementa un enfoque híbrido que combina el almacenamiento de grafos de conocimiento (RDF) con índices vectoriales de alta dimensión (HNSW). Este sistema permite no solo la recuperación de información semánticamente relevante, sino también la inferencia lógica deductiva mediante reglas RDFS y OWL-RL, garantizando la consistencia y la trazabilidad de la información a través de un modelo de procedencia robusto.

## 1. Introducción

Los Modelos de Lenguaje Grande (LLMs) han demostrado capacidades excepcionales en el procesamiento del lenguaje natural, pero sufren de limitaciones inherentes relacionadas con su ventana de contexto finita y la alucinación de hechos. Los enfoques convencionales para mitigar esto, como RAG, suelen fragmentar el texto en vectores, perdiendo las relaciones estructurales y lógicas entre las entidades.

Synapse aborda este problema proponiendo una arquitectura de "Memoria Semántica" que unifica dos paradigmas:
1.  **Simbólico:** Representación explícita del conocimiento mediante grafos (sujeto-predicado-objeto).
2.  **Conexionista:** Representación latente mediante incrustaciones vectoriales (embeddings).

El objetivo es proporcionar a los agentes de IA un sustrato de memoria que sea consultable tanto por similitud semántica como por estructura lógica.

## 2. Arquitectura del Sistema

El núcleo de Synapse está implementado en **Rust**, priorizando la seguridad de memoria, la concurrencia y el rendimiento. El sistema se expone a través de interfaces gRPC para la comunicación interna y el **Model Context Protocol (MCP)** para la integración estandarizada con agentes de IA.

Los componentes principales son:
*   **Motor de Ingesta:** Procesa texto crudo y tripletas RDF, gestionando la normalización y la asignación de identificadores únicos (URIs).
*   **Almacén Híbrido (SynapseStore):** Orquesta la persistencia dual en grafo y vector.
*   **Razonador Semántico:** Aplica reglas de inferencia para materializar conocimiento implícito.
*   **Interfaz de Búsqueda Híbrida:** Combina resultados vectoriales con expansión de grafos.

## 3. Almacenamiento Híbrido y Persistencia

Synapse implementa un mecanismo de almacenamiento dual que garantiza la consistencia atómica entre el grafo y el índice vectorial.

### 3.1. Almacén de Grafos (RDF)
Utiliza **Oxigraph** como motor de base de datos embebido compatible con SPARQL 1.1. Los datos se almacenan en formato de cuádruplas (Sujeto, Predicado, Objeto, Grafo).
Un aspecto distintivo es el modelo de **Procedencia (Provenance)**. Cada operación de escritura genera un identificador de lote (UUID) que se utiliza como nombre del grafo (`urn:batch:{uuid}`). Los metadatos de procedencia (fuente, fecha, método de extracción) se almacenan en el grafo por defecto, vinculados a este identificador de lote, permitiendo una trazabilidad completa del origen de cada dato.

### 3.2. Almacén Vectorial (HNSW)
Implementa un índice **HNSW (Hierarchical Navigable Small World)** personalizado, utilizando la distancia euclidiana para medir la similitud.
*   **Modelo de Embedding:** Se utiliza el modelo `sentence-transformers/all-MiniLM-L6-v2`, que genera vectores de 384 dimensiones.
*   **Sincronización:** El sistema emplea un patrón de transacción compensatoria. Durante la ingesta, los datos se escriben primero en el grafo. Si la posterior inserción en el índice vectorial falla, la operación en el grafo se revierte automáticamente, asegurando que no existan nodos en el grafo que no sean recuperables vectorialmente.

## 4. Motor de Razonamiento (Inferencia)

A diferencia de las bases de datos vectoriales pasivas, Synapse incorpora un razonador activo (`SynapseReasoner`) que materializa inferencias lógicas basadas en ontologías estándar.

El sistema soporta múltiples estrategias de razonamiento configurables:
1.  **RDFS (RDF Schema):** Implementa la transitividad de `subClassOf`. Si $A \subseteq B$ y $B \subseteq C$, el sistema infiere y persiste $A \subseteq C$.
2.  **OWL-RL (Web Ontology Language - Rule Language):** Soporta un subconjunto eficiente de reglas OWL, incluyendo:
    *   **Propiedades Transitivas:** Si $p$ es transitiva y $x \xrightarrow{p} y \xrightarrow{p} z$, se infiere $x \xrightarrow{p} z$.
    *   **Propiedades Simétricas:** Si $p$ es simétrica y $x \xrightarrow{p} y$, se infiere $y \xrightarrow{p} x$.
    *   **Propiedades Inversas:** Si $p_1$ es inversa de $p_2$ y $x \xrightarrow{p_1} y$, se infiere $y \xrightarrow{p_2} x$.

El proceso de materialización utiliza un bucle de punto fijo que aplica estas reglas iterativamente hasta que no se generan nuevas tripletas, garantizando que todas las implicaciones lógicas estén disponibles para la consulta en tiempo real.

## 5. Recuperación de Información Híbrida

El mecanismo de búsqueda de Synapse supera las limitaciones de la búsqueda vectorial pura mediante un algoritmo de dos fases:

1.  **Búsqueda Vectorial Inicial:** Se recuperan los $k$ nodos más similares semánticamente a la consulta del usuario utilizando el índice HNSW.
2.  **Expansión de Grafo (Graph Traversal):** A partir de los nodos recuperados, el sistema realiza una expansión transversal (BFS) en el grafo de conocimiento. Esto permite recuperar el contexto estructural (vecinos directos) de los conceptos hallados.

El resultado final es una lista ponderada que prioriza la similitud semántica pero enriquece el contexto con relaciones explícitas que podrían no estar capturadas en la proximidad vectorial latente.

## 6. Protocolos e Interoperabilidad

Synapse adopta el **Model Context Protocol (MCP)**, un estándar emergente para la conexión de modelos de IA con fuentes de datos externas. A través de MCP, Synapse expone herramientas ("tools") que permiten a un LLM:
*   Insertar conocimientos (tripletas) explícitamente.
*   Realizar consultas SPARQL complejas.
*   Ejecutar búsquedas híbridas en lenguaje natural.

Esto desacopla la lógica de almacenamiento de la lógica del agente, permitiendo que Synapse actúe como un "lóbulo temporal" modular para cualquier arquitectura de IA compatible.

## 7. Conclusión

Synapse representa un avance significativo hacia sistemas de IA neuro-simbólicos. Al combinar la flexibilidad de los embeddings vectoriales con la precisión y la estructura de los grafos de conocimiento RDF, ofrece una solución robusta al problema de la memoria a largo plazo. Su implementación en Rust y su capacidad de razonamiento deductivo lo posicionan como una infraestructura crítica para el desarrollo de agentes autónomos confiables y explicables.
