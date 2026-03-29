use std::env;
use std::sync::Arc;
use synapse_core::mcp::server::start_mcp_server;
use synapse_core::mcp::stdio::run_stdio_mcp_server;
use synapse_core::store::SynapseStore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let is_stdio = args.contains(&"--stdio".to_string());

    let storage_path = env::var("GRAPH_STORAGE_PATH").unwrap_or_else(|_| "data/graphs".to_string());

    let store = Arc::new(SynapseStore::open("default", &storage_path)?);

    if is_stdio {
        eprintln!("Synapse-MCP starting in stdio mode...");
        run_stdio_mcp_server(store).await;
    } else {
        println!(
            r#"

  _________.__. ____ _____  ______  ______ ____
 /  ___<   |  |/    \\__  \ \____ \/  ___// __ \
 \___ \ \___  |   |  \/ __ \|  |_> >___ \\  ___/
/____  >/ ____|___|  (____  /   __/____  >\___  >
     \/ \/         \/     \/|__|       \/     \/
"#
        );
        let port = env::var("SYNAPSE_MCP_PORT").unwrap_or_else(|_| "3000".to_string()).parse()?;
        println!("🚀 Synapse Engine starting (Pure Symbolic Mode, MCP Ext-Apps ready)");
        println!("Storage Path: {}", storage_path);

        start_mcp_server(port, store).await;
    }

    Ok(())
}
