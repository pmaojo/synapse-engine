# Synapse Deployment Guide

Synapse is designed to run as a persistent, long-running Model Context Protocol (MCP) server. Its core is a stateful Rust application backed by Oxigraph and RocksDB.

## Why Not Vercel?

Deploying the `synapse-core` Rust backend to serverless platforms like Vercel or AWS Lambda is **highly discouraged** and structurally incompatible for several reasons:

1.  **Ephemeral Filesystem**: Vercel functions are stateless and their filesystems are destroyed after execution. Synapse uses RocksDB to store the symbolic knowledge graph on disk. If deployed to Vercel, the graph would be completely wiped clean on every cold start.
2.  **Execution Model**: Synapse is designed to listen persistently via HTTP/SSE (`axum`) or standard I/O for local MCP clients (like Claude Desktop). Serverless functions only run per-request and have strict execution timeout limits (e.g., 10 to 60 seconds).
3.  **Stateful Reasoning**: The OWL-RL fixed-point reasoning engine materializes facts in memory and flushes them to disk. This requires continuous execution time and a persistent environment.

## Recommended Architecture

To deploy Synapse for remote access (e.g., using the HTTP/SSE transport for remote AI agents), you need a hosting provider that supports **long-running Docker containers with attached persistent volumes**.

Excellent choices include:
*   **Fly.io**
*   **Railway.app**
*   **Render**

---

### Backend Deployment (Fly.io Example)

Fly.io is an excellent choice for hosting Synapse because it supports Dockerfiles and persistent storage volumes natively.

**1. Install `flyctl` and Login**
```bash
curl -L https://fly.io/install.sh | sh
flyctl auth login
```

**2. Initialize the App**
In the root of the repository (where the `Dockerfile` is located):
```bash
flyctl launch
```
*   Follow the prompts to choose an app name and region.
*   **Do not deploy yet** when it asks. We need to attach a volume first.

**3. Create a Persistent Volume**
Create a volume to store the RocksDB graph data:
```bash
flyctl volumes create synapse_data --size 1
```

**4. Update `fly.toml`**
Edit the generated `fly.toml` to mount the volume. By default, the `synapse-core` engine looks for a `data/` directory relative to its execution path or via the `GRAPH_STORAGE_PATH` environment variable.

```toml
app = "your-synapse-app-name"
primary_region = "iad"

[env]
  GRAPH_STORAGE_PATH = "/app/data"
  RUST_LOG = "info"

[mounts]
  source = "synapse_data"
  destination = "/app/data"

[http_service]
  internal_port = 3000
  force_https = true
  auto_stop_machines = false # Important: Keep the engine running
  auto_start_machines = true
  min_machines_running = 1
```

**5. Deploy**
```bash
flyctl deploy
```

---

### Frontend Ext-App Deployment (Optional)

We have built a React + Vite frontend for the MCP Ext-App UI. By default, our build scripts bundle this frontend into a single `index.html` file using `vite-plugin-singlefile`, which the Rust backend serves natively when an AI client requests the `ui://` resources.

**Local vs. Hosted UI**
*   **Local MCP (Default)**: You don't need to host the frontend separately. The Rust server reads `frontend/dist/index.html` and serves it directly to the host client (e.g., Claude Desktop).
*   **Hosted UI (Vercel)**: If you prefer to host the dashboard publicly or separately, you *can* deploy the `frontend/` folder to Vercel.

**Deploying Frontend to Vercel:**
1.  Connect your GitHub repository to Vercel.
2.  Set the Framework Preset to **Vite**.
3.  Set the Root Directory to `frontend`.
4.  Leave the Build Command as `npm run build`.
5.  Deploy.

*Note: The frontend UI requires the `@modelcontextprotocol/ext-apps` SDK and expects to be rendered inside an iframe by an MCP-compliant host client to function correctly.*