# Kthulu Go Mastering Scenario

This scenario provides semantic knowledge about **Kthulu Go**, an AI‑Native Software Foundry for Go.

## Purpose

Enable neuro‑symbolic reasoning about Kthulu concepts, commands, and best practices. The triples in this scenario can be queried via SPARQL or hybrid search to:

- Recall CLI command syntax and parameters.
- Understand the architectural style (Vertical Slice, Modular Monolith).
- Relate technologies (Go, Templ, HTMX, Godog, Gherkin).
- Support automated scaffolding, BDD test generation, and project analysis.

## Ontology Overview

### Core Classes

- **KthuluProject** – A Modular Monolith project.
- **KthuluModule** – A vertical slice (feature module).
- **KthuluFeature** – A BDD feature file.
- **KthuluScenario** – A concrete scenario within a feature.
- **KthuluCommand** – A CLI command (`create`, `add module`, `bdd run`, etc.).
- **KthuluParameter** – A command parameter (`name`, `fields`, `features`).
- **KthuluTechnology** – A technology used in the stack.
- **KthuluArchitectureStyle** – Architectural pattern (Vertical Slice, Modular Monolith).

### Relationships

- `hasModule` – Project → Module.
- `hasFeature` – Module → Feature.
- `hasScenario` – Feature → Scenario.
- `hasCommand` – Project → Command.
- `usesTechnology` – Project → Technology.
- `followsArchitectureStyle` – Project → ArchitectureStyle.
- `requiresParameter` – Command → Parameter.

### Data Properties

- `commandSyntax` – CLI syntax string.
- `description` – Human‑readable description.
- `example` – Example usage.

## Usage Examples

### SPARQL Query: List all commands

```sparql
PREFIX kthulu: <http://synapse.os/kthulu#>
SELECT ?cmd ?syn ?desc
WHERE {
  ?cmd a kthulu:Command .
  ?cmd kthulu:commandSyntax ?syn .
  ?cmd kthulu:description ?desc .
}
```

### Hybrid Search

Use `hybrid_search` with natural‑language queries like:

- “How do I create a new Kthulu project?”
- “What parameters does `add module` accept?”
- “Which technologies are used in Kthulu?”

## Integration with OpenClaw

When OpenClaw needs to answer questions about Kthulu, it can query this scenario via Synapse MCP (`sparql_query` or `hybrid_search`). The retrieved triples provide factual context for generating accurate, structured answers.

## Extending the Scenario

To add more knowledge:

1. Add new triples to `data/commands.ttl` or `data/modules.ttl`.
2. Extend the ontology in `schema/kthulu.owl` if new classes/properties are needed.
3. Update `manifest.json` to include new data files.

## References

- [Kthulu Skill](/skills/kthulu/SKILL.md)
- [Kthulu CLI Quick Reference](/skills/kthulu/SKILL.md#cli-quick-reference)
- [Synapse Scenario Marketplace](/skills/synapse/SKILL.md#scenario-marketplace)