use crate::iri;
use crate::store::SynapseStore;
use oxigraph::model::{Literal, Term, Triple};
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
                    "name": "assert_facts",
                    "description": ASSERT_FACTS_DESCRIPTION,
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "facts": {
                                "type": "array",
                                "minItems": 1,
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "subject": { "type": "string", "description": "Entity id (people/bob), CURIE (schema:x) or full IRI" },
                                        "predicate": { "type": "string", "description": "Relation name (works_at), 'a' for rdf:type, CURIE (rdfs:label) or full IRI" },
                                        "object": { "type": "string" },
                                        "object_type": { "type": "string", "enum": ["entity", "literal"], "description": "Default: 'entity' when predicate is 'a'/rdf:type or object is a full IRI; otherwise 'literal'" },
                                        "datatype": { "type": "string", "description": "Literal datatype, e.g. xsd:integer, xsd:date" },
                                        "lang": { "type": "string", "description": "Literal language tag, e.g. es" }
                                    },
                                    "required": ["subject", "predicate", "object"]
                                }
                            },
                            "source": { "type": "string", "description": "Who is asserting (agent or person name). Default: 'agent'" },
                            "reason": { "type": "string", "description": "Why these facts are true (stored as provenance)" }
                        },
                        "required": ["facts"]
                    }
                },
                {
                    "name": "retract_facts",
                    "description": "Retract facts previously asserted, either a whole assertion (assertion_id) or individual facts (same shape as assert_facts). Facts stated by a Markdown/ontology source are not removed: edit the source and re-index it. Every retraction is logged with its reason in the provenance graph.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "assertion_id": { "type": "string", "description": "IRI returned by assert_facts (urn:synapse:assertion:…)" },
                            "facts": { "type": "array", "items": { "type": "object" } },
                            "source": { "type": "string" },
                            "reason": { "type": "string" }
                        }
                    }
                },
                {
                    "name": "index_markdown_directory",
                    "description": "Index (or re-index) a folder of Markdown files, e.g. an OKF bundle. Each file is one entity whose id is its path relative to the folder without extension (people/bob.md -> people/bob); [[target]], [[rel:target]] and [[rel::target]] become relations; frontmatter fields become properties ('type' -> rdf:type). Re-indexing replaces what changed files said and drops files that were deleted.",
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
                    "description": "Fetch the provenance of a graph: an assertion id, a source graph, or a prefix of either (as shown in [Prov: …] tags).",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string" }
                        },
                        "required": ["id"]
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
                    let id = args["id"].as_str().or_else(|| args["hash"].as_str()).ok_or("Missing id")?;
                    let id = id.trim().trim_start_matches('<').trim_end_matches('>');
                    let prefix = if iri::is_absolute(id) { id.to_string() } else { format!("{}{}", iri::ASSERTION_NS, id) };
                    let meta = iri::node(iri::META_GRAPH);
                    let mut lines: Vec<String> = store
                        .store
                        .quads_for_pattern(None, None, None, Some(meta.as_ref().into()))
                        .flatten()
                        .filter(|q| match &q.subject {
                            oxigraph::model::Subject::NamedNode(n) => n.as_str().starts_with(&prefix),
                            _ => false,
                        })
                        .map(|q| format!("{} {} {}", q.subject, iri::short(q.predicate.as_str()), q.object))
                        .collect();
                    lines.sort();
                    let text = if lines.is_empty() {
                        format!("No provenance found for {}", id)
                    } else {
                        format!("Provenance Trace for {}:\n{}", id, lines.join("\n"))
                    };
                    Ok(json!({ "content": [ { "type": "text", "text": text } ] }))
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
                    ", iri::class(slice_name).map_err(|e| e.to_string())?.as_str());

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
                    let component_iri = iri::entity(component_name).map_err(|e| e.to_string())?;
                    let mut union_blocks = Vec::new();

                    for d in 1..=depth {
                        let mut block = String::new();
                        block.push_str(&format!("?s1 ?p1 <{}> . \n", component_iri));
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
                        match engine.ingest_file(path, Some("CoreSpecification")).await {
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
                    let entity_iri = iri::entity(entity_id).map_err(|e| e.to_string())?;
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
                    ", entity_iri.as_str(), entity_iri.as_str());

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
                    ", entity_iri.as_str(), entity_iri.as_str());

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

                                    let prov_hash = prov_tag(g);

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

                                    let prov_hash = prov_tag(g);

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
                    use crate::ingest::IngestionEngine;

                    let dir_path = args["directory_path"].as_str().ok_or("Missing directory_path")?;
                    let engine = IngestionEngine::new(store.clone());
                    let report = engine
                        .index_markdown_directory(std::path::Path::new(dir_path))
                        .await
                        .map_err(|e| e.to_string())?;
                    let mut text = format!(
                        "Indexed {}: {} changed file(s) ({} triples), {} unchanged, {} removed.",
                        dir_path, report.changed, report.triples, report.unchanged, report.removed
                    );
                    if !report.errors.is_empty() {
                        text.push_str(&format!("\nErrors:\n{}", report.errors.join("\n")));
                    }
                    Ok(json!({ "content": [ { "type": "text", "text": text } ] }))
                }
                "assert_facts" => {
                    let facts = parse_facts(&args["facts"])?;
                    let source = args["source"].as_str().unwrap_or("agent");
                    let out = store
                        .assert_facts(&facts, source, args["reason"].as_str())
                        .map_err(|e| e.to_string())?;
                    let listed: Vec<String> = facts.iter().map(|t| format!("{t} .")).collect();
                    Ok(json!({
                        "content": [ { "type": "text", "text": format!(
                            "Asserted {} fact(s) as {} ({} new, {} already supported by other sources):\n{}",
                            facts.len(), out.assertion.as_str(), out.inserted - out.already_known.min(out.inserted), out.already_known, listed.join("\n")
                        ) } ]
                    }))
                }
                "retract_facts" => {
                    let source = args["source"].as_str().unwrap_or("agent");
                    let reason = args["reason"].as_str();
                    let out = if let Some(id) = args["assertion_id"].as_str() {
                        let graph = iri::entity(id).map_err(|e| e.to_string())?;
                        store.retract_assertion(&graph, source, reason)
                    } else if args.get("facts").is_some() {
                        store.retract_facts(&parse_facts(&args["facts"])?, source, reason)
                    } else {
                        return Err("Provide assertion_id or facts".to_string());
                    }
                    .map_err(|e| e.to_string())?;

                    let mut text = match &out.retraction {
                        Some(r) => format!("Retracted {} fact(s) (logged as {}).", out.removed.len(), r.as_str()),
                        None => "Nothing retracted.".to_string(),
                    };
                    for (t, g) in &out.owned_by_sources {
                        text.push_str(&format!("\nNot retracted, stated by source {}: {} .", g.as_str(), t));
                    }
                    for t in &out.not_found {
                        text.push_str(&format!("\nNot found: {} .", t));
                    }
                    Ok(json!({ "content": [ { "type": "text", "text": text } ] }))
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
                "version": env!("CARGO_PKG_VERSION")
            }
        })),
        "notifications/initialized" => Ok(json!({})),
        _ => Err(format!("Method not supported: {}", method)),
    }
}

const ASSERT_FACTS_DESCRIPTION: &str = "Record facts in long-term memory as one assertion with provenance (source, reason, time). Use stable ids: the same id always means the same entity, and Markdown files indexed with index_markdown_directory use their relative path as id (people/bob). Returns the assertion id, which retract_facts accepts.";

/// Parses the `facts` argument of assert_facts / retract_facts.
fn parse_facts(value: &Value) -> Result<Vec<Triple>, String> {
    let items = value.as_array().ok_or("'facts' must be an array")?;
    if items.is_empty() {
        return Err("'facts' is empty".to_string());
    }
    items
        .iter()
        .enumerate()
        .map(|(i, f)| parse_fact(f).map_err(|e| format!("facts[{}]: {}", i, e)))
        .collect()
}

fn parse_fact(f: &Value) -> Result<Triple, String> {
    let field = |k: &str| f[k].as_str().map(str::trim).filter(|s| !s.is_empty()).ok_or(format!("missing '{}'", k));
    let subject = iri::entity(field("subject")?).map_err(|e| e.to_string())?;
    let predicate = iri::predicate(field("predicate")?).map_err(|e| e.to_string())?;
    let object = f["object"].as_str().ok_or("missing 'object'")?;
    let is_type = predicate.as_str() == iri::RDF_TYPE;

    let as_entity = match f["object_type"].as_str() {
        Some("entity") | Some("iri") => true,
        Some("literal") => false,
        Some(other) => return Err(format!("unknown object_type '{}'", other)),
        None => is_type || iri::is_absolute(object.trim()),
    };

    let object: Term = if as_entity {
        if is_type { iri::class(object) } else { iri::entity(object) }.map_err(|e| e.to_string())?.into()
    } else if let Some(lang) = f["lang"].as_str() {
        Literal::new_language_tagged_literal(object, lang).map_err(|e| e.to_string())?.into()
    } else if let Some(dt) = f["datatype"].as_str() {
        Literal::new_typed_literal(object, iri::datatype(dt).map_err(|e| e.to_string())?).into()
    } else {
        Literal::new_simple_literal(object).into()
    };
    Ok(Triple::new(subject, predicate, object))
}

/// `[Prov: …]` tag for a graph IRI as returned by SPARQL (`<…>`).
fn prov_tag(graph: &str) -> String {
    let g = graph.trim_start_matches('<').trim_end_matches('>');
    if let Some(id) = g.strip_prefix(iri::ASSERTION_NS) {
        format!(" [Prov: {}]", &id[..id.len().min(8)])
    } else if let Some(src) = g.strip_prefix(iri::SOURCE_NS) {
        let file = src.strip_prefix("file:").and_then(|p| iri::file_path(&format!("file://{}", p))).unwrap_or_else(|| src.to_string());
        format!(" [Source: {}]", file.rsplit('/').next().unwrap_or(&file))
    } else {
        String::new()
    }
}
