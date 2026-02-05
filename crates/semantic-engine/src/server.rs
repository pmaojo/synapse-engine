use std::sync::Arc;
use tonic::{Request, Response, Status};
use dashmap::DashMap;

pub mod proto {
    tonic::include_proto!("semantic_engine");
}

use proto::semantic_engine_server::SemanticEngine;
use proto::*;

use crate::store::SynapseStore;
use crate::reasoner::{SynapseReasoner, ReasoningStrategy as InternalStrategy};
use crate::server::proto::{ReasoningStrategy, SearchMode};
use crate::ingest::IngestionEngine;
use std::path::Path;

pub struct MySemanticEngine {
    pub storage_path: String,
    pub stores: DashMap<String, Arc<SynapseStore>>,
}

impl MySemanticEngine {
    pub fn new(storage_path: &str) -> Self {
        Self {
            storage_path: storage_path.to_string(),
            stores: DashMap::new(),
        }
    }

    fn get_store(&self, namespace: &str) -> Result<Arc<SynapseStore>, Status> {
        if let Some(store) = self.stores.get(namespace) {
            return Ok(store.clone());
        }

        let store = SynapseStore::open(namespace, &self.storage_path)
            .map_err(|e| Status::internal(format!("Failed to open store for namespace '{}': {}", namespace, e)))?;
        
        let store_arc = Arc::new(store);
        self.stores.insert(namespace.to_string(), store_arc.clone());
        Ok(store_arc)
    }
}

#[tonic::async_trait]
impl SemanticEngine for MySemanticEngine {
    async fn ingest_triples(
        &self,
        request: Request<IngestRequest>,
    ) -> Result<Response<IngestResponse>, Status> {
        let req = request.into_inner();
        let namespace = if req.namespace.is_empty() { "default" } else { &req.namespace };
        let store = self.get_store(namespace)?;

        let triples: Vec<(String, String, String)> = req
            .triples
            .into_iter()
            .map(|t| (t.subject, t.predicate, t.object))
            .collect();

        match store.ingest_triples(triples).await {
            Ok((added, _)) => Ok(Response::new(IngestResponse {
                nodes_added: added,
                edges_added: added,
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn ingest_file(
        &self,
        request: Request<IngestFileRequest>,
    ) -> Result<Response<IngestResponse>, Status> {
        let req = request.into_inner();
        let namespace = if req.namespace.is_empty() { "default" } else { &req.namespace };
        let store = self.get_store(namespace)?;
        
        let engine = IngestionEngine::new(store);
        let path = Path::new(&req.file_path);

        match engine.ingest_file(path, namespace).await {
            Ok(count) => Ok(Response::new(IngestResponse {
                nodes_added: count,
                edges_added: count,
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn get_neighbors(
        &self,
        _request: Request<NodeRequest>,
    ) -> Result<Response<NeighborResponse>, Status> {
        Ok(Response::new(NeighborResponse { neighbors: vec![] }))
    }

    async fn search(
        &self,
        request: Request<SearchRequest>,
    ) -> Result<Response<SearchResponse>, Status> {
        let req = request.into_inner();
        let namespace = if req.namespace.is_empty() { "default" } else { &req.namespace };
        let store = self.get_store(namespace)?;

        match store.hybrid_search(&req.query, req.limit as usize, 0).await {
            Ok(results) => {
                let grpc_results = results
                    .into_iter()
                    .enumerate()
                    .map(|(idx, (uri, score))| SearchResult {
                        node_id: idx as u32,
                        score,
                        content: uri.clone(),
                        uri,
                    })
                    .collect();
                Ok(Response::new(SearchResponse { results: grpc_results }))
            }
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn resolve_id(
        &self,
        _request: Request<ResolveRequest>,
    ) -> Result<Response<ResolveResponse>, Status> {
        Ok(Response::new(ResolveResponse {
            node_id: 0,
            found: false,
        }))
    }

    async fn get_all_triples(
        &self,
        request: Request<EmptyRequest>,
    ) -> Result<Response<TriplesResponse>, Status> {
        let req = request.into_inner();
        let namespace = if req.namespace.is_empty() { "default" } else { &req.namespace };
        let store = self.get_store(namespace)?;
        
        let mut triples = Vec::new();

        for quad in store.store.iter().map(|q| q.unwrap()) {
            triples.push(Triple {
                subject: quad.subject.to_string(),
                predicate: quad.predicate.to_string(),
                object: quad.object.to_string(),
                provenance: Some(Provenance {
                    source: "oxigraph".to_string(),
                    timestamp: "".to_string(),
                    method: "storage".to_string(),
                }),
                embedding: vec![],
            });
        }

        Ok(Response::new(TriplesResponse { triples }))
    }

    async fn query_sparql(
        &self,
        request: Request<SparqlRequest>,
    ) -> Result<Response<SparqlResponse>, Status> {
        let req = request.into_inner();
        let namespace = if req.namespace.is_empty() { "default" } else { &req.namespace };
        let store = self.get_store(namespace)?;
        
        match store.query_sparql(&req.query) {
            Ok(json) => Ok(Response::new(SparqlResponse { results_json: json })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn delete_namespace_data(
        &self,
        request: Request<EmptyRequest>,
    ) -> Result<Response<DeleteResponse>, Status> {
        let req = request.into_inner();
        let namespace = if req.namespace.is_empty() { "default" } else { &req.namespace };
        
        // Remove from cache
        self.stores.remove(namespace);
        
        // Delete directory
        let path = Path::new(&self.storage_path).join(namespace);
        if path.exists() {
            std::fs::remove_dir_all(path).map_err(|e| Status::internal(e.to_string()))?;
        }

        Ok(Response::new(DeleteResponse {
            success: true,
            message: format!("Deleted namespace '{}'", namespace),
        }))
    }

    async fn hybrid_search(
        &self,
        request: Request<HybridSearchRequest>,
    ) -> Result<Response<SearchResponse>, Status> {
        let req = request.into_inner();
        let namespace = if req.namespace.is_empty() { "default" } else { &req.namespace };
        let store = self.get_store(namespace)?;

        let vector_k = req.vector_k as usize;
        let graph_depth = req.graph_depth;

        let results = match SearchMode::try_from(req.mode) {
            Ok(SearchMode::VectorOnly) | Ok(SearchMode::Hybrid) => {
                store.hybrid_search(&req.query, vector_k, graph_depth).await
                    .map_err(|e| Status::internal(format!("Hybrid search failed: {}", e)))?
            }
            _ => vec![],
        };

        let grpc_results = results
            .into_iter()
            .enumerate()
            .map(|(idx, (uri, score))| SearchResult {
                node_id: idx as u32,
                score,
                content: uri.clone(),
                uri,
            })
            .collect();

        Ok(Response::new(SearchResponse { results: grpc_results }))
    }

    async fn apply_reasoning(
        &self,
        request: Request<ReasoningRequest>,
    ) -> Result<Response<ReasoningResponse>, Status> {
        let req = request.into_inner();
        let namespace = if req.namespace.is_empty() { "default" } else { &req.namespace };
        let store = self.get_store(namespace)?;
        
        let strategy = match ReasoningStrategy::try_from(req.strategy) {
            Ok(ReasoningStrategy::Rdfs) => InternalStrategy::RDFS,
            Ok(ReasoningStrategy::Owlrl) => InternalStrategy::OWLRL,
            _ => InternalStrategy::None,
        };

        let reasoner = SynapseReasoner::new(strategy);
        
        if req.materialize {
            match reasoner.materialize(&store.store) {
                Ok(count) => Ok(Response::new(ReasoningResponse {
                    success: true,
                    triples_inferred: count as u32,
                    message: format!("Materialized {} triples in namespace '{}'", count, namespace),
                })),
                Err(e) => Err(Status::internal(e.to_string())),
            }
        } else {
            match reasoner.apply(&store.store) {
                Ok(triples) => Ok(Response::new(ReasoningResponse {
                    success: true,
                    triples_inferred: triples.len() as u32,
                    message: format!("Found {} inferred triples in namespace '{}'", triples.len(), namespace),
                })),
                Err(e) => Err(Status::internal(e.to_string())),
            }
        }
    }
}

pub async fn run_mcp_stdio(engine: Arc<MySemanticEngine>) -> Result<(), Box<dyn std::error::Error>> {
    let server = crate::mcp_stdio::McpStdioServer::new(engine);
    server.run().await
}
