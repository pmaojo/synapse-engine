use std::fs;
use std::path::Path;

fn get_frontend_dist_path() -> String {
    // Attempt to locate the frontend dist folder
    // In production, this might be bundled or at a fixed relative path
    let potential_paths = vec![
        "../../frontend/dist/index.html",
        "../frontend/dist/index.html",
        "frontend/dist/index.html",
        "/app/frontend/dist/index.html", // Sandbox absolute
    ];

    for p in potential_paths {
        if Path::new(p).exists() {
            return p.to_string();
        }
    }

    // Fallback: Just try to read it from the CWD
    "frontend/dist/index.html".to_string()
}

pub fn generate_graph_html(entity_uri: &str) -> String {
    // We are using vite-plugin-singlefile, so index.html contains all CSS/JS inline.
    // We don't need to parse the manifest, just read the HTML and inject the routing path if needed.

    let path = get_frontend_dist_path();
    let html_content = fs::read_to_string(&path).unwrap_or_else(|_| {
        format!("<h1>Error: Could not load frontend dist file at {}</h1>", path)
    });

    // The react-router uses the URL path, but in an iframe data URI or similar MCP context,
    // it might be difficult to pass the path. We can inject a small script to mock the window.location
    // or we can just rely on the query params we pass in the `src` attribute if the host supports it.

    // A trick to make react-router-dom start at the right location in an isolated environment:
    let injection = format!(
        r#"
        <script>
            // Set the initial hash for the React HashRouter
            window.location.hash = '#/graph?uri=' + encodeURIComponent("{}");
        </script>
        </head>
        "#,
        entity_uri
    );

    html_content.replace("</head>", &injection)
}

pub fn generate_dashboard_html() -> String {
    let path = get_frontend_dist_path();
    let html_content = fs::read_to_string(&path).unwrap_or_else(|_| {
        format!("<h1>Error: Could not load frontend dist file at {}</h1>", path)
    });

    let injection = r#"
        <script>
            window.location.hash = '#/';
        </script>
        </head>
    "#;

    html_content.replace("</head>", injection)
}
