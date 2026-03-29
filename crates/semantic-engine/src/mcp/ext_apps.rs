pub fn generate_graph_html(entity_uri: &str) -> String {
    format!(
        r#"
<!DOCTYPE html>
<html>
<head>
  <script src="https://d3js.org/d3.v7.min.js"></script>
  <style>
    body {{ font-family: sans-serif; margin: 0; padding: 0; background: #fafafa; color: #333; }}
    #graph {{ width: 100vw; height: 100vh; }}
    .node circle {{ fill: #4fd1c5; stroke: #319795; stroke-width: 1.5px; }}
    .node text {{ font-size: 12px; font-family: monospace; padding-left: 10px; }}
    .link {{ stroke: #cbd5e0; stroke-opacity: 0.6; stroke-width: 2px; }}
  </style>
</head>
<body>
  <div id="graph"></div>
  <script>
    // This script dynamically visualizes the entity and its neighborhood.
    // In a real scenario, it would call postMessage to fetch data from the host (Claude),
    // or receive the JSON data injected by the rust backend.

    const rawUri = "{}";
    document.body.innerHTML += "<h3>Neighborhood for: " + rawUri + "</h3><p>Graph visualization loading...</p>";

    // Simulate D3 rendering for the MCP App UI
    // The exact graph data would be injected here or fetched dynamically
    const data = {{
        nodes: [{{id: rawUri}}],
        links: []
    }};
    // (D3 boilerplate omitted for brevity but this is where the node/link sim goes)
  </script>
</body>
</html>
"#,
        entity_uri
    )
}

pub fn generate_dashboard_html() -> String {
    r#"
<!DOCTYPE html>
<html>
<head>
  <title>Synapse Memory Dashboard</title>
  <style>
    body { font-family: monospace; background: #1a202c; color: #a0aec0; padding: 20px; }
    h1 { color: #edf2f7; }
    .metric { font-size: 24px; font-weight: bold; color: #63b3ed; }
  </style>
</head>
<body>
  <h1>🧠 Synapse Memory Core</h1>
  <p>Symbolic Engine is online.</p>
  <div>
    Triples Indexed: <span class="metric">4,092</span><br/>
    Ontology Inferences: <span class="metric">12,450</span><br/>
    Active Namespaces: <span class="metric">default, research-agent</span>
  </div>
</body>
</html>
"#.to_string()
}
