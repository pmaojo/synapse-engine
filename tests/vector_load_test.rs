use synapse_core::vector_store::VectorStore;
use std::env;

#[tokio::main]
async fn main() {
    env::set_var("GRAPH_STORAGE_PATH", "/home/robin/data/graphs");
    env::set_var("FASTEMBED_CACHE_PATH", "/home/robin/.cache/fastembed");
    
    let namespace = "kthulu-go";
    println!("Testing vector load for namespace '{}'...", namespace);
    
    match VectorStore::new(namespace) {
        Ok(vs) => {
            let (active, stale, total) = vs.stats();
            println!("Stats: active={}, stale={}, total={}", active, stale, total);
        },
        Err(e) => println!("Error: {}", e),
    }
}
