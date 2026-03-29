# MCP & Ext-Apps Integration Status

## Current Status

The `synapse-core` project currently implements a Model Context Protocol (MCP) server with some foundational steps taken toward supporting the new "Ext-Apps" (MCP Apps) specification.

### Implemented Features
1. **Dual Transport**: The server successfully implements both HTTP/SSE (`mcp/server.rs`) and Standard I/O (`mcp/stdio.rs`) transports for MCP.
2. **Tools**: It exposes tools like `sparql_query`, `get_entity_neighborhood`, and `index_markdown_directory`.
3. **Resource Exposing**: It exposes UI resources in `resources/list`, specifically `ui://synapse/dashboard` and `ui://synapse/graph/{entity_uri}`.
4. **Tool to UI Linkage**: When `get_entity_neighborhood` is called, it returns a text response along with a `resource` reference in the content array (`{ "type": "resource", "resource": { "uri": "ui://..." } }`), which is the correct conceptual pattern for instructing an MCP Apps-compliant host to render a UI.
5. **HTML Generation**: It dynamically generates HTML templates (`ext_apps.rs`) returned when `resources/read` is called for the `ui://` URIs.

## Gaps & Missing Capabilities

While the scaffolding is there, the implementation falls short of a fully compliant and functional "MCP App" according to the official `@modelcontextprotocol/ext-apps` specification.

### 1. Missing the MCP Apps SDK / Bidirectional Communication
- **Current State**: The HTML generated in `ext_apps.rs` is essentially static boilerplate. It contains a hardcoded D3.js script that notes: *"In a real scenario, it would call postMessage to fetch data from the host (Claude), or receive the JSON data injected by the rust backend."*
- **The Gap**: An official MCP App requires bidirectional communication with the host client (e.g., Claude, ChatGPT). This is usually achieved by importing `@modelcontextprotocol/ext-apps` (or implementing the underlying `postMessage` protocol manually) inside the returned HTML. Without this, the UI cannot receive the actual graph data dynamically from the host or trigger subsequent tool calls from within the UI.

### 2. Missing Real Data Injection
- **Current State**: `get_entity_neighborhood` expands the graph and gets the neighbors, but the returned HTML resource template (`generate_graph_html`) does not actually receive this expanded data. It just hardcodes `nodes: [{id: rawUri}]`.
- **The Gap**: The host client expects to use the MCP Apps protocol to send a notification (e.g., `window.parent.postMessage`) to the iframe containing the context/data of the tool call. The HTML needs a script that listens for this data and updates the D3 visualization accordingly.

### 3. Missing `mimeType` in Tool Response
- **Current State**: The `get_entity_neighborhood` returns `{ "type": "resource", "resource": { "uri": interactive_ui_uri } }`.
- **The Gap**: While it works, it is best practice (and sometimes required by clients) to include the `mimeType` (e.g., `"text/html"`) alongside the `uri` in the resource object returned by the tool, so the client immediately knows it can be rendered as an iframe.

### 4. Limited Ext-Apps Capabilities
- **Current State**: The server only implements `resources/list` and `resources/read` for the Ext-Apps.
- **The Gap**: The MCP Apps specification allows UIs to call back into the server (via the host). Our server doesn't provide any specific endpoints or tools designed *for* the UI to consume interactively (e.g., clicking a node in the UI to fetch *its* neighborhood).

## Conclusion
The server acts as a standard MCP server with some custom `ui://` resources. It is **not yet a fully compliant MCP Ext-App**. To bridge the gap, the HTML templates in `ext_apps.rs` must be updated to implement the Ext-Apps `postMessage` protocol (or bundle a lightweight JS client) so they can dynamically receive data from the host client and render it.
