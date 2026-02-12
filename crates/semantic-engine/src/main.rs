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

    // Load default ontologies on startup
    let ontology_path = std::path::Path::new("ontology");
    if ontology_path.exists() {
        let msg = format!("Loading ontologies from {:?}", ontology_path);
        if is_mcp {
            eprintln!("{}", msg);
        } else {
            println!("{}", msg);
        }

        match engine.get_store("default") {
            Ok(store) => {
                match synapse_core::ingest::ontology::OntologyLoader::load_directory(
                    &store,
                    ontology_path,
                )
                .await
                {
                    Ok(count) => {
                        let msg =
                            format!("Loaded {} ontology triples into 'default' namespace", count);
                        if is_mcp {
                            eprintln!("{}", msg);
                        } else {
                            println!("{}", msg);
                        }
                    }
                    Err(e) => eprintln!("Failed to load ontologies: {}", e),
                }
            }
            Err(e) => eprintln!("Failed to open default store for ontologies: {}", e),
        }
    } else if !is_mcp {
        println!("No 'ontology' directory found, skipping ontology loading.");
    }

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
