use crate::store::SynapseStore;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::RwLock;
use tonic::{Request, Response, Status};

// Import the generated proto code
pub mod semantic_engine {
    tonic::include_proto!("semantic_engine");
}

use semantic_engine::semantic_engine_server::SemanticEngine;
use semantic_engine::{
    DeleteResponse, EmptyRequest, IngestRequest, IngestResponse, Neighbor, NeighborResponse,
    NodeRequest, Provenance, ResolveRequest, ResolveResponse, SearchRequest, SearchResponse,
    Triple, TriplesResponse, SparqlRequest, SparqlResponse,
};

pub struct MySemanticEngine {
    pub namespaces: Arc<RwLock<HashMap<String, Arc<SynapseStore>>>>,
    pub storage_path: Arc<String>,
}

impl Clone for MySemanticEngine {
    fn clone(&self) -> Self {
        Self {
            namespaces: Arc::clone(&self.namespaces),
            storage_path: Arc::clone(&self.storage_path),
        }
    }
}

impl MySemanticEngine {
    pub fn new(storage_path: &str) -> Self {
        if !Path::new(storage_path).exists() {
            std::fs::create_dir_all(storage_path).unwrap();
        }

        Self {
            namespaces: Arc::new(RwLock::new(HashMap::new())),
            storage_path: Arc::new(storage_path.to_string()),
        }
    }

    fn get_namespace_store(&self, namespace: &str) -> Result<Arc<SynapseStore>, Status> {
        let ns = if namespace.is_empty() { "default" } else { namespace };

        {
            let namespaces = self.namespaces.read().unwrap();
            if let Some(store) = namespaces.get(ns) {
                return Ok(store.clone());
            }
        }

        let mut namespaces = self.namespaces.write().unwrap();
        if let Some(store) = namespaces.get(ns) {
            return Ok(store.clone());
        }

        let store = SynapseStore::open(ns, &self.storage_path)
            .map_err(|e| Status::internal(format!("Failed to open store: {}", e)))?;
        let store_arc = Arc::new(store);
        namespaces.insert(ns.to_string(), store_arc.clone());
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
        let store = self.get_namespace_store(&req.namespace)?;

        let triples: Vec<(String, String, String)> = req.triples.into_iter()
            .map(|t| (t.subject, t.predicate, t.object))
            .collect();

        match store.ingest_triples(triples) {
            Ok((added, _)) => Ok(Response::new(IngestResponse {
                nodes_added: added,
                edges_added: added,
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn get_neighbors(
        &self,
        request: Request<NodeRequest>,
    ) -> Result<Response<NeighborResponse>, Status> {
        // Implementation for traversal...
        Ok(Response::new(NeighborResponse { neighbors: vec![] }))
    }

    async fn search(
        &self,
        _request: Request<SearchRequest>,
    ) -> Result<Response<SearchResponse>, Status> {
        Ok(Response::new(SearchResponse { results: vec![] }))
    }

    async fn resolve_id(
        &self,
        request: Request<ResolveRequest>,
    ) -> Result<Response<ResolveResponse>, Status> {
        let req = request.into_inner();
        let store = self.get_namespace_store(&req.namespace)?;
        let id = store.get_or_create_id(&req.content);
        Ok(Response::new(ResolveResponse {
            node_id: id,
            found: true,
        }))
    }

    async fn get_all_triples(
        &self,
        request: Request<EmptyRequest>,
    ) -> Result<Response<TriplesResponse>, Status> {
        let req = request.into_inner();
        let store = self.get_namespace_store(&req.namespace)?;

        let mut triples = Vec::new();
        // Fetch from Oxigraph...
        for quad in store.store.iter().map(|q| q.unwrap()) {
            triples.push(Triple {
                subject: format!("{}", quad.subject),
                predicate: format!("{}", quad.predicate),
                object: format!("{}", quad.object),
                provenance: None,
            });
        }

        Ok(Response::new(TriplesResponse { triples }))
    }

    async fn query_sparql(
        &self,
        request: Request<SparqlRequest>,
    ) -> Result<Response<SparqlResponse>, Status> {
        let req = request.into_inner();
        let store = self.get_namespace_store(&req.namespace)?;

        match store.store.query(&req.query) {
            Ok(results) => {
                let mut output = String::new();
                match results {
                    oxigraph::sparql::QueryResults::Solutions(solutions) => {
                        for solution in solutions {
                            let s = solution.unwrap();
                            output.push_str(&format!("{:?}\n", s));
                        }
                    }
                    oxigraph::sparql::QueryResults::Boolean(v) => {
                        output = format!("{}", v);
                    }
                    oxigraph::sparql::QueryResults::Graph(quads) => {
                        for quad in quads {
                            output.push_str(&format!("{:?}\n", quad.unwrap()));
                        }
                    }
                }
                Ok(Response::new(SparqlResponse {
                    results_json: output,
                }))
            }
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn delete_namespace_data(
        &self,
        request: Request<EmptyRequest>,
    ) -> Result<Response<DeleteResponse>, Status> {
        let req = request.into_inner();
        let ns = req.namespace;

        {
            let mut namespaces = self.namespaces.write().unwrap();
            namespaces.remove(&ns);
        }

        Ok(Response::new(DeleteResponse {
            success: true,
            message: format!("Deleted namespace {}", ns),
        }))
    }
}
