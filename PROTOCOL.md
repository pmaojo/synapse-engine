# PROTOCOLO DE OPERACIONES DE SYNAPSE (POS) 🧠⛓️

Este protocolo define cómo Robin gestiona la memoria a largo plazo y la estructura del conocimiento del sistema.

## 1. EL BUCLE DEL BIBLIOTECARIO (Ingesta de Conocimiento)
**Objetivo:** Capturar hechos atómicos de la actividad diaria.
- **Cuándo:** Al finalizar cada hito, tras leer archivos de configuración o cuando el usuario declare un hecho importante.
- **Proceso:**
    1. Identificar Entidades (Sujetos y Objetos).
    2. Identificar Relaciones (Predicados).
    3. Validar contra la Ontología actual.
    4. Inyectar en el motor Synapse (Rust) vía MCP.

## 2. EL BUCLE DEL ARQUITECTO (Mantenimiento de Ontología)
**Objetivo:** Asegurar que el "vocabulario" del sistema es suficiente y coherente.
- **Cuándo:** Cuando Robin detecta una entidad o relación que no encaja en las clases actuales de `scenarios/core/schema/`.
- **Proceso:**
    1. Proponer nueva Clase o Propiedad OWL.
    2. Verificar jerarquía (subClassOf) para mantener la herencia de razonamiento.
    3. Actualizar archivos `.owl`.
    4. Recargar el grafo en el motor de Rust.

## 3. EL BUCLE DEL ANALISTA (Consulta y Razonamiento)
**Objetivo:** Usar el conocimiento para mejorar la toma de decisiones.
- **Cuándo:** Al inicio de cualquier tarea compleja (Fase 1 del PRI).
- **Proceso:**
    1. Consultar Synapse: "¿Qué sabemos sobre este componente/tecnología/requisito?".
    2. Realizar inferencia: "¿Hay relaciones implícitas que afecten a este cambio?".
    3. Inyectar los resultados en el contexto del LLM para una respuesta precisa.

---
*Robin - Memoria Estructurada, Inteligencia Implacable.*
