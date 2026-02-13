use synapse_core::server::proto::semantic_engine_client::SemanticEngineClient;
use synapse_core::server::proto::{
    HybridSearchRequest, IngestRequest, Provenance, ReasoningRequest, ReasoningStrategy,
    SearchMode, Triple,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to Synapse gRPC server...");
    let mut client = SemanticEngineClient::connect("http://[::1]:50051").await?;

    println!("✅ Connected!");

    // 1. Ingest Data
    let triple = Triple {
        subject: "http://example.org/Socrates".to_string(),
        predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
        object: "http://example.org/Human".to_string(),
        provenance: Some(Provenance {
            source: "client_test".to_string(),
            timestamp: "now".to_string(),
            method: "grpc".to_string(),
        }),
        embedding: vec![],
    };

    let triple2 = Triple {
        subject: "http://example.org/Human".to_string(),
        predicate: "http://www.w3.org/2000/01/rdf-schema#subClassOf".to_string(),
        object: "http://example.org/Mortal".to_string(),
        provenance: Some(Provenance {
            source: "client_test".to_string(),
            timestamp: "now".to_string(),
            method: "grpc".to_string(),
        }),
        embedding: vec![],
    };

    println!("Sending IngestRequest...");
    let response = client
        .ingest_triples(IngestRequest {
            triples: vec![triple, triple2],
            namespace: "test_verification".to_string(),
        })
        .await?;
    println!("Response: {:?}", response.into_inner());

    // 2. Apply Reasoning (RDFS Transitivity)
    println!("\nApplying RDFS Reasoning (Internal)...");
    let reasoning_response = client
        .apply_reasoning(ReasoningRequest {
            namespace: "test_verification".to_string(),
            strategy: ReasoningStrategy::Rdfs as i32,
            materialize: false,
        })
        .await?;
    println!("Reasoning Result: {:?}", reasoning_response.into_inner());

    // 3. Hybrid Search
    println!("\nPerforming Hybrid Search for 'Socrates'...");
    let search_response = client
        .hybrid_search(HybridSearchRequest {
            query: "Socrates".to_string(),
            namespace: "test_verification".to_string(),
            vector_k: 5,
            graph_depth: 1,
            mode: SearchMode::Hybrid as i32,
            limit: 10,
        })
        .await?;

    println!("Search Results:");
    for result in search_response.into_inner().results {
        println!(
            " - [Score: {:.4}] {} ({})",
            result.score, result.content, result.uri
        );
    }

    Ok(())
}
