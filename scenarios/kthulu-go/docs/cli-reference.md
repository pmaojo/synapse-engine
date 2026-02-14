# Kthulu CLI Quick Reference

## Core Scaffolding

| Command | Syntax | Description | Example |
|---------|--------|-------------|---------|
| `create` | `kthulu create [name] --features=auth,user,billing` | Scaffold a new production‑ready Modular Monolith. | `kthulu create myapp --features=auth,user` |
| `add module` | `kthulu add module [name] [fields...]` | Add a new vertical slice to an existing project. | `kthulu add module products name:string price:float` |
| `plan` | `kthulu plan [name] --template=ecommerce` | Define the system blueprint in `kthulu‑plan.yaml`. | `kthulu plan mysystem --template=ecommerce` |

## BDD & Testing

| Command | Syntax | Description | Example |
|---------|--------|-------------|---------|
| `bdd run` | `kthulu bdd run [filter]` | Execute all scenarios or filter by name/regex. | `kthulu bdd run @smoke` |
| `bdd list` | `kthulu bdd list` | Find all defined features in the project. | `kthulu bdd list` |
| `ai gen‑feature` | `kthulu ai gen‑feature "description" --apply` | Use AI to generate a feature and its implementation. | `kthulu ai gen‑feature "User login flow" --apply` |

## Analysis & Audit

| Command | Syntax | Description | Example |
|---------|--------|-------------|---------|
| `analyze overview` | `kthulu analyze overview` | Get a high‑level overview of the project architecture. | `kthulu analyze overview` |
| `analyze modules` | `kthulu analyze modules` | List all vertical slices (modules) and their structure. | `kthulu analyze modules` |
| `analyze deps` | `kthulu analyze deps` | Visualize and audit internal/external dependencies between modules. | `kthulu analyze deps` |
| `audit` | `kthulu audit` | Security check. | `kthulu audit` |
| `audit security` | `kthulu audit --security --compliance=gdpr` | Run a security and compliance audit (SOX, GDPR, PCI). | `kthulu audit --security --compliance=gdpr` |

## Database Migrations

| Command | Syntax | Description | Example |
|---------|--------|-------------|---------|
| `migrate create` | `kthulu migrate create [name]` | Create a new database migration. | `kthulu migrate create add_users_table` |
| `migrate up` | `kthulu migrate up` | Apply pending database migrations. | `kthulu migrate up` |
| `migrate status` | `kthulu migrate status` | Show status of database migrations. | `kthulu migrate status` |

## Development

| Command | Syntax | Description | Example |
|---------|--------|-------------|---------|
| `dev` | `kthulu dev` | Run the development server with hot‑reloading and AI self‑healing. | `kthulu dev` |
| `coder` | `kthulu coder` | Launch AI agent. | `kthulu coder` |
| `ai` | `kthulu ai "prompt" --apply` | Run one‑off AI commands. | `kthulu ai "Add Stripe webhook handler" --apply` |

## Common Parameters

- `[name]` – Project or module name.
- `[fields...]` – Field definitions in format `field:type` (e.g., `name:string price:float`).
- `--features` – Comma‑separated list of features to include (`auth`, `user`, `billing`, etc.).
- `--template` – Blueprint template name (`ecommerce`, `saas`, `internal‑tool`).
- `[filter]` – Regex filter for scenarios (e.g., `@smoke`, `@wip`).
- `--security` – Enable security audit.
- `--compliance` – Compliance standard (`gdpr`, `sox`, `pci`).

## Architecture & Stack

- **Vertical Slice Architecture** – Code organised by feature, not technical layer.
- **GTH Stack** – Go + Templ + HTMX.
- **BDD** – Gherkin + Godog for behavior‑driven development.
- **Modular Monolith** – Loosely coupled modules inside a single binary.

## MCP Integration

Kthulu exposes its CLI as an MCP server. Use `mcporter` to call tools like `project_overview`, `add_module`, `bdd_run`, etc.

Example:
```bash
mcporter call --stdio "kthulu mcp" project_overview
```

## Best Practices

- Define your architecture in a plan before scaffolding.
- Keep shared kernels in `internal/infrastructure`.
- Use tags for grouping BDD scenarios (`@smoke`, `@wip`).
- Run `kthulu audit` regularly for security checks.