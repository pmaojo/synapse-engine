use tonic::transport::Server;
use semantic_engine::server::{MySemanticEngine, semantic_engine::semantic_engine_server::SemanticEngineServer};
use semantic_engine::mcp_stdio::run_mcp_stdio;
use std::env;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let is_mcp = args.contains(&"--mcp".to_string());

    // Get storage path from env or default
    let storage_path = env::var("GRAPH_STORAGE_PATH").unwrap_or_else(|_| "data/graphs".to_string());

    let engine = MySemanticEngine::new(&storage_path);

    if is_mcp {
        println!("🚀 Starting Synapse-MCP (stdio mode)...");
        run_mcp_stdio(Arc::new(engine)).await?;
    } else {
        let addr = "[::1]:50051".parse()?;
        println!("🚀 Synapse (ex-Grafoso) listening on {}", addr);
        println!("Storage Path: {}", storage_path);

        Server::builder()
            .add_service(SemanticEngineServer::new(engine))
            .serve(addr)
            .await?;
    }

    Ok(())
}
