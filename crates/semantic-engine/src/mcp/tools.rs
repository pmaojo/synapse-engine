use crate::store::SynapseStore;
use serde_json::{json, Value};
use std::sync::Arc;

pub async fn handle_tool_call(
    method: &str,
    params: Option<Value>,
    store: Arc<SynapseStore>,
) -> Result<Value, String> {
    match method {
        "tools/list" => Ok(json!({
            "tools": [
                {
                    "name": "sparql_query",
                    "description": "Execute a SPARQL query against the symbolic knowledge graph.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": { "type": "string" }
                        },
                        "required": ["query"]
                    }
                },
                {
                    "name": "get_entity_neighborhood",
                    "description": "Expand the graph around an entity to see connections. Returns interactive UI view.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "uri": { "type": "string" },
                            "depth": { "type": "number", "default": 1 }
                        },
                        "required": ["uri"]
                    }
                },
                {
                    "name": "index_markdown_directory",
                    "description": "Trigger ingestion of a folder containing markdown files to sync with the graph.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "directory_path": { "type": "string", "description": "Absolute path to the directory" }
                        },
                        "required": ["directory_path"]
                    }
                }
            ]
        })),
        "tools/call" => {
            let p = params.ok_or("Missing params")?;
            let name = p["name"].as_str().ok_or("Missing tool name")?;
            let args = p.get("arguments").cloned().unwrap_or(json!({}));

            match name {
                "sparql_query" => {
                    let query = args["query"].as_str().ok_or("Missing query")?;
                    match store.query_sparql(query) {
                        Ok(res) => Ok(json!({
                            "content": [
                                { "type": "text", "text": res }
                            ]
                        })),
                        Err(e) => Err(format!("SPARQL Error: {}", e)),
                    }
                }
                "get_entity_neighborhood" => {
                    let uri = args["uri"].as_str().ok_or("Missing uri")?;
                    let depth = args["depth"].as_u64().unwrap_or(1) as u32;

                    // Graph expansion
                    match store.expand_graph(uri, depth) {
                        Ok(neighbors) => {
                            // UI Resource generation for MCP Apps
                            let interactive_ui_uri = format!("ui://synapse/graph/{}", urlencoding::encode(uri));

                            // To be fully MCP Apps compliant, the tool should return standard text content
                            // *and* the embedded view references. But simply returning the raw text is
                            // baseline MCP. Ext-Apps can fetch the resource.
                            let text_output = serde_json::to_string_pretty(&neighbors).unwrap();
                            Ok(json!({
                                "content": [
                                    { "type": "text", "text": format!("Neighborhood of {}:\n{}", uri, text_output) },
                                    { "type": "resource", "resource": { "uri": interactive_ui_uri } }
                                ]
                            }))
                        }
                        Err(e) => Err(format!("Graph expansion error: {}", e)),
                    }
                }
                "index_markdown_directory" => {
                    use walkdir::WalkDir;

                    use crate::ingest::IngestionEngine;

                    let dir_path = args["directory_path"].as_str().ok_or("Missing directory_path")?;
                    let engine = IngestionEngine::new(store.clone());
                    let mut total_added = 0;

                    for entry in WalkDir::new(dir_path).into_iter().filter_map(|e| e.ok()) {
                        let path = entry.path();
                        if path.is_file() {
                            if let Some(ext) = path.extension() {
                                if ext == "md" || ext == "markdown" {
                                    match engine.ingest_file(path, "default").await {
                                        Ok(count) => total_added += count,
                                        Err(e) => eprintln!("Failed to index file {}: {}", path.display(), e),
                                    }
                                }
                            }
                        }
                    }

                    Ok(json!({
                        "content": [
                            { "type": "text", "text": format!("Successfully indexed {} new triples from markdown files in {}", total_added, dir_path) }
                        ]
                    }))
                }
                _ => Err(format!("Unknown tool: {}", name)),
            }
        }
        "resources/list" => {
             Ok(json!({
                "resources": [
                    {
                        "uri": "ui://synapse/dashboard",
                        "name": "Synapse Global Dashboard",
                        "mimeType": "text/html",
                        "description": "Interactive global view of the memory engine."
                    },
                    {
                        "uri": "ui://synapse/graph/{entity_uri}",
                        "name": "Visualización del Subgrafo",
                        "mimeType": "text/html",
                        "description": "Plantilla para renderizar interactivamente el vecindario (BFS) de una entidad."
                    }
                ]
            }))
        }
        "resources/read" => {
            let p = params.ok_or("Missing params")?;
            let uri = p["uri"].as_str().ok_or("Missing uri")?;

            if uri.starts_with("ui://synapse/graph/") {
                // Return an interactive HTML snippet with d3.js or similar to visualize the subgraph.
                // For MCP Apps, the host renders this inside a sandboxed iframe.
                let entity_uri = urlencoding::decode(&uri[19..]).unwrap_or_default();
                let html = crate::mcp::ext_apps::generate_graph_html(&entity_uri);
                Ok(json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "text/html",
                        "text": html
                    }]
                }))
            } else if uri == "ui://synapse/dashboard" {
                 let html = crate::mcp::ext_apps::generate_dashboard_html();
                 Ok(json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "text/html",
                        "text": html
                    }]
                }))
            } else {
                Err(format!("Resource not found: {}", uri))
            }
        }
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": { "listChanged": false },
                "resources": { "subscribe": true }
            },
            "serverInfo": {
                "name": "synapse-core-mcp",
                "version": "0.9.0"
            }
        })),
        "notifications/initialized" => Ok(json!({})),
        _ => Err(format!("Method not supported: {}", method)),
    }
}
