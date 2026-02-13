use std::env;
use std::sync::Arc;
use synapse_core::server::{
    proto::semantic_engine_server::SemanticEngineServer, run_mcp_stdio, MySemanticEngine,
};
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let is_mcp = args.contains(&"--mcp".to_string());

    // Get storage path from env or default
    let storage_path = env::var("GRAPH_STORAGE_PATH").unwrap_or_else(|_| "data/graphs".to_string());

    let engine = MySemanticEngine::new(&storage_path);

    // Ensure 'core' scenario is installed on startup (backgrounded for MCP performance)
    let engine_init = engine.clone();
    tokio::spawn(async move {
        match engine_init.install_scenario("core", "default").await {
            Ok(msg) => eprintln!("{}", msg),
            Err(e) => eprintln!("Failed to load core scenario: {}", e),
        }
    });

    if is_mcp {
        // MCP mode: no stdout messages, only JSON-RPC
        eprintln!("Synapse-MCP starting (stdio mode)...");
        run_mcp_stdio(Arc::new(engine)).await?;
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
        let addr = "[::1]:50051".parse()?;
        println!("🚀 Synapse (ex-Grafoso) listening on {}", addr);
        println!("Storage Path: {}", storage_path);

        let engine_clone = engine.clone();

        Server::builder()
            .add_service(SemanticEngineServer::with_interceptor(
                engine,
                synapse_core::server::auth_interceptor,
            ))
            .serve_with_shutdown(addr, async move {
                if tokio::signal::ctrl_c().await.is_ok() {
                    println!("\nShutting down Synapse...");
                }
                engine_clone.shutdown().await;
            })
            .await?;
    }

    Ok(())
}
