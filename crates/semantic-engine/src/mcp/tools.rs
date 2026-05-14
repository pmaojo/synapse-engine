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
                    "name": "get_entity_narrative",
                    "description": "Fetch a distilled Markdown summary (dossier) of an entity, including its identity, attributes, and key relationships.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "entity_id": { "type": "string" }
                        },
                        "required": ["entity_id"]
                    }
                },
                {
                    "name": "get_domain_logic",
                    "description": "Fetch all entities and relationships bounded by a specific domain's aggregate root.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "slice_name": { "type": "string" }
                        },
                        "required": ["slice_name"]
                    }
                },
                {
                    "name": "get_dependency_impact",
                    "description": "Traverse structural dependencies up to a bounded depth to determine the impact of a component.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "component_name": { "type": "string" },
                            "depth": { "type": "number", "default": 1 }
                        },
                        "required": ["component_name"]
                    }
                },
                {
                    "name": "sync_specification_to_graph",
                    "description": "Ingest a markdown file as a Core Specification, granting it elevated truth status.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "spec_file_path": { "type": "string" }
                        },
                        "required": ["spec_file_path"]
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
                },
                {
                    "name": "sparql_query",
                    "description": "[Graph Admin Only] Execute a raw SPARQL query. Use this only for debugging or if semantic macros fail.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": { "type": "string" }
                        },
                        "required": ["query"]
                    }
                },
                {
                    "name": "get_provenance",
                    "description": "Fetch the full provenance trace for a specific provenance hash snippet.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "hash": { "type": "string" }
                        },
                        "required": ["hash"]
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
                "get_provenance" => {
                    let hash = args["hash"].as_str().ok_or("Missing hash")?;
                    let hash_str = if hash.starts_with("urn:batch:") {
                        hash.to_string()
                    } else {
                        format!("urn:batch:{}", hash)
                    };

                    // Use FILTER(STRSTARTS) to handle truncated hashes correctly.
                    // Provenance triples are stored in the default graph for easy querying.
                    let query = format!("
                        PREFIX prov: <http://www.w3.org/ns/prov#>
                        SELECT ?graph ?p ?o
                        WHERE {{
                            ?graph ?p ?o .
                            FILTER(STRSTARTS(STR(?graph), \"{}\"))
                        }}
                    ", hash_str);

                    match store.query_sparql(&query) {
                        Ok(res) => Ok(json!({
                            "content": [
                                { "type": "text", "text": format!("Provenance Trace for {}:\n{}", hash, res) }
                            ]
                        })),
                        Err(e) => Err(format!("SPARQL Error: {}", e)),
                    }
                }
                "get_domain_logic" => {
                    let slice_name = args["slice_name"].as_str().ok_or("Missing slice_name")?;
                    let query = format!("
                        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
                        SELECT ?s ?p ?o
                        WHERE {{
                            ?s ?p ?o .
                            ?s rdfs:subClassOf* <{}> .
                        }}
                    ", store.ensure_uri(slice_name));

                    match store.query_sparql(&query) {
                        Ok(res) => Ok(json!({
                            "content": [
                                { "type": "text", "text": format!("Domain Logic for {}:\n{}", slice_name, res) }
                            ]
                        })),
                        Err(e) => Err(format!("SPARQL Error: {}", e)),
                    }
                }
                "get_dependency_impact" => {
                    let component_name = args["component_name"].as_str().ok_or("Missing component_name")?;
                    let depth = args.get("depth").and_then(|v| v.as_u64()).unwrap_or(1);

                    // Since dynamic depth and unbounded variable property paths (?p+) are invalid in standard SPARQL 1.1,
                    // we simulate depth by executing a programmatic BFS locally or constructing explicit UNION blocks
                    // For the sake of standard SPARQL compatibility, we construct a pattern of explicit depth joins up to the bound
                    let mut union_blocks = Vec::new();

                    for d in 1..=depth {
                        let mut block = String::new();
                        block.push_str(&format!("?s1 ?p1 <{}> . \n", store.ensure_uri(component_name)));
                        if d > 1 {
                            for i in 2..=d {
                                block.push_str(&format!("?s{} ?p{} ?s{} . \n", i, i, i - 1));
                            }
                        }
                        // Select the furthest subject and its property
                        block.push_str(&format!("BIND(?s{} AS ?impacted_node)\n", d));
                        union_blocks.push(format!("{{ {} }}", block));
                    }

                    let query = format!("
                        SELECT DISTINCT ?impacted_node
                        WHERE {{
                            {}
                        }}
                    ", union_blocks.join(" UNION "));

                    match store.query_sparql(&query) {
                        Ok(res) => Ok(json!({
                            "content": [
                                { "type": "text", "text": format!("Dependency Impact for {} (depth {}):\n{}", component_name, depth, res) }
                            ]
                        })),
                        Err(e) => Err(format!("SPARQL Error: {}", e)),
                    }
                }
                "sync_specification_to_graph" => {
                    use crate::ingest::IngestionEngine;
                    let spec_file_path = args["spec_file_path"].as_str().ok_or("Missing spec_file_path")?;
                    let engine = IngestionEngine::new(store.clone());

                    let path = std::path::Path::new(spec_file_path);
                    if path.is_file() {
                        match engine.ingest_file(path, "CoreSpecification").await {
                            Ok(count) => Ok(json!({
                                "content": [
                                    { "type": "text", "text": format!("Successfully ingested {} new triples from specification file {}", count, spec_file_path) }
                                ]
                            })),
                            Err(e) => Err(format!("Failed to index file {}: {}", path.display(), e)),
                        }
                    } else {
                        Err(format!("File not found: {}", spec_file_path))
                    }
                }
                "get_entity_narrative" => {
                    let entity_id = args["entity_id"].as_str().ok_or("Missing entity_id")?;
                    // We add ORDER BY ?time DESC to get adaptive context window based on recency
                    // Core properties (like type) are stored in the default graph, so we query without GRAPH restriction,
                    // and use OPTIONAL GRAPH to get provenance metadata if available.
                    let query = format!("
                        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
                        PREFIX prov: <http://www.w3.org/ns/prov#>
                        PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>

                        SELECT ?p ?o ?g ?time
                        WHERE {{
                            <{}> ?p ?o .
                            OPTIONAL {{
                                GRAPH ?g {{ <{}> ?p ?o . }}
                                ?g prov:generatedAtTime ?time .
                            }}
                        }}
                        ORDER BY DESC(?time)
                    ", store.ensure_uri(entity_id), store.ensure_uri(entity_id));

                    let inbound_query = format!("
                        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
                        PREFIX prov: <http://www.w3.org/ns/prov#>
                        PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>

                        SELECT ?s ?p ?g ?time
                        WHERE {{
                            ?s ?p <{}> .
                            OPTIONAL {{
                                GRAPH ?g {{ ?s ?p <{}> . }}
                                ?g prov:generatedAtTime ?time .
                            }}
                        }}
                        ORDER BY DESC(?time)
                    ", store.ensure_uri(entity_id), store.ensure_uri(entity_id));

                    let mut type_val = String::from("Unknown");
                    let mut attributes = Vec::new();
                    let mut outbound = Vec::new();
                    let mut inbound = Vec::new();
                    let mut recent_context = String::new();

                    if let Ok(res_str) = store.query_sparql(&query) {
                        if let Ok(res) = serde_json::from_str::<Value>(&res_str) {
                            if let Value::Array(arr) = res {
                                for row in arr.iter().take(20) { // Limit to top 20 edges for context window
                                    let p = row.get("p").and_then(|v| v.as_str()).unwrap_or("").trim_matches('"');
                                    let o = row.get("o").and_then(|v| v.as_str()).unwrap_or("");
                                    let is_literal = o.starts_with('"');
                                    let o_clean = o.trim_matches('"');
                                    let time = row.get("time").and_then(|v| v.as_str()).unwrap_or("").trim_matches('"');
                                    let g = row.get("g").and_then(|v| v.as_str()).unwrap_or("").trim_matches('"');

                                    let prov_hash = if g.starts_with("urn:batch:") {
                                        let hash_part = &g["urn:batch:".len()..];
                                        let short_hash = if hash_part.len() > 8 { &hash_part[..8] } else { hash_part };
                                        format!(" [Prov: {}]", short_hash)
                                    } else {
                                        String::new()
                                    };

                                    if p.ends_with("type") {
                                        type_val = o_clean.to_string();
                                    } else if is_literal {
                                        attributes.push(format!("- **{}**: {}{}", p.split(&['/', '#'][..]).last().unwrap_or(p), o_clean, prov_hash));
                                    } else {
                                        outbound.push(format!("- **{}** -> {}{}", p.split(&['/', '#'][..]).last().unwrap_or(p), o_clean, prov_hash));
                                    }

                                    if time > recent_context.as_str() {
                                        recent_context = time.to_string();
                                    }
                                }
                            }
                        }
                    }

                    if let Ok(res_str) = store.query_sparql(&inbound_query) {
                        if let Ok(res) = serde_json::from_str::<Value>(&res_str) {
                            if let Value::Array(arr) = res {
                                for row in arr.iter().take(20) { // Limit to top 20 edges for context window
                                    let s = row.get("s").and_then(|v| v.as_str()).unwrap_or("").trim_matches('"');
                                    let p = row.get("p").and_then(|v| v.as_str()).unwrap_or("").trim_matches('"');
                                    let time = row.get("time").and_then(|v| v.as_str()).unwrap_or("").trim_matches('"');
                                    let g = row.get("g").and_then(|v| v.as_str()).unwrap_or("").trim_matches('"');

                                    let prov_hash = if g.starts_with("urn:batch:") {
                                        let hash_part = &g["urn:batch:".len()..];
                                        let short_hash = if hash_part.len() > 8 { &hash_part[..8] } else { hash_part };
                                        format!(" [Prov: {}]", short_hash)
                                    } else {
                                        String::new()
                                    };

                                    inbound.push(format!("- {} -> **{}**{}", s, p.split(&['/', '#'][..]).last().unwrap_or(p), prov_hash));
                                    if time > recent_context.as_str() {
                                        recent_context = time.to_string();
                                    }
                                }
                            }
                        }
                    }

                    let narrative = format!(
                        "# Dossier: {}\n\n**Identity**: {}\n\n### Attributes\n{}\n\n### Relationships\n#### Outbound (Relies On)\n{}\n\n#### Inbound (Depended Upon By)\n{}\n\n### Recent Context\nLatest Activity: {}",
                        entity_id,
                        type_val,
                        if attributes.is_empty() { "None".to_string() } else { attributes.join("\n") },
                        if outbound.is_empty() { "None".to_string() } else { outbound.join("\n") },
                        if inbound.is_empty() { "None".to_string() } else { inbound.join("\n") },
                        if recent_context.is_empty() { "Unknown" } else { &recent_context }
                    );

                    Ok(json!({
                        "content": [
                            { "type": "text", "text": narrative }
                        ]
                    }))
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
                                    { "type": "resource", "resource": { "uri": interactive_ui_uri, "mimeType": "text/html" } }
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
