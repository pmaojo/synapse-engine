use dashmap::DashMap;
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub mod proto {
    tonic::include_proto!("semantic_engine");
}

use proto::semantic_engine_server::SemanticEngine;
use proto::*;

use crate::ingest::IngestionEngine;
use crate::reasoner::{ReasoningStrategy as InternalStrategy, SynapseReasoner};
use crate::scenarios::ScenarioManager;
use crate::server::proto::{ReasoningStrategy, SearchMode};
use crate::store::{IngestTriple, SynapseStore};
use std::path::Path;

use crate::audit::InferenceAudit;
use crate::auth::NamespaceAuth;

#[derive(Clone)]
pub struct AuthToken(pub String);

#[allow(clippy::result_large_err)]
pub fn auth_interceptor(mut req: Request<()>) -> Result<Request<()>, Status> {
    if let Some(token) = req
        .metadata()
        .get("authorization")
        .and_then(|t| t.to_str().ok())
        .map(|s| s.trim_start_matches("Bearer ").to_string())
    {
        req.extensions_mut().insert(AuthToken(token));
    }
    Ok(req)
}

fn get_token<T>(req: &Request<T>) -> Option<String> {
    if let Some(token) = req.extensions().get::<AuthToken>() {
        return Some(token.0.clone());
    }
    req.metadata()
        .get("authorization")
        .and_then(|t| t.to_str().ok())
        .map(|s| s.trim_start_matches("Bearer ").to_string())
}

#[derive(Clone)]
pub struct MySemanticEngine {
    pub storage_path: String,
    pub stores: Arc<DashMap<String, Arc<SynapseStore>>>,
    pub auth: Arc<NamespaceAuth>,
    pub audit: Arc<InferenceAudit>,
    pub scenario_manager: Arc<ScenarioManager>,
}

impl MySemanticEngine {
    pub fn new(storage_path: &str) -> Self {
        let auth = Arc::new(NamespaceAuth::new());
        auth.load_from_env();
        let scenario_manager = Arc::new(ScenarioManager::new(std::path::Path::new(".")));

        Self {
            storage_path: storage_path.to_string(),
            stores: Arc::new(DashMap::new()),
            auth,
            audit: Arc::new(InferenceAudit::new()),
            scenario_manager,
        }
    }

    pub async fn install_scenario(&self, name: &str, namespace: &str) -> Result<String, String> {
        let path = self
            .scenario_manager
            .install_scenario(name)
            .await
            .map_err(|e| format!("Failed to install scenario assets: {}", e))?;

        let store = self
            .get_store(namespace)
            .map_err(|e| e.message().to_string())?;

        // Load Ontologies
        let schema_path = path.join("schema");
        let mut triples_loaded = 0;
        if schema_path.exists() {
            triples_loaded +=
                crate::ingest::ontology::OntologyLoader::load_directory(&store, &schema_path)
                    .await
                    .map_err(|e| format!("Failed to load ontologies: {}", e))?;
        }

        // Load Data (Files)
        let data_path = path.join("data");
        let mut data_files_loaded = 0;
        if data_path.exists() {
            if let Ok(entries) = std::fs::read_dir(data_path) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() {
                        // Use ingestion engine
                        let engine = IngestionEngine::new(store.clone());
                        if let Ok(count) = engine.ingest_file(&p, namespace).await {
                            triples_loaded += count as usize;
                            data_files_loaded += 1;
                        }
                    }
                }
            }
        }

        // Load Docs (RAG)
        let docs_path = path.join("docs");
        let mut docs_loaded = 0;
        if docs_path.exists() {
            if let Ok(entries) = std::fs::read_dir(docs_path) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() {
                        if let Ok(content) = std::fs::read_to_string(&p) {
                            let processor = crate::processor::TextProcessor::new();
                            let chunks = processor.chunk_text(&content, 1000, 150);
                            if let Some(ref vector_store) = store.vector_store {
                                for (i, chunk) in chunks.iter().enumerate() {
                                    let chunk_uri = format!("file://{}#chunk-{}", p.display(), i);
                                    let metadata = serde_json::json!({
                                        "uri": format!("file://{}", p.display()),
                                        "type": "doc_chunk",
                                        "scenario": name
                                    });
                                    let _ = vector_store.add(&chunk_uri, chunk, metadata).await;
                                }
                                docs_loaded += 1;
                            }
                        }
                    }
                }
            }
        }

        Ok(format!(
            "Scenario '{}' installed. Loaded {} triples ({} data files) and {} docs.",
            name, triples_loaded, data_files_loaded, docs_loaded
        ))
    }

    pub async fn shutdown(&self) {
        eprintln!("Shutting down... flushing {} stores", self.stores.len());
        for entry in self.stores.iter() {
            let store = entry.value();
            if let Err(e) = store.flush() {
                eprintln!("Failed to flush store '{}': {}", entry.key(), e);
            }
        }
        eprintln!("Shutdown complete.");
    }

    #[allow(clippy::result_large_err)]
    pub fn get_store(&self, namespace: &str) -> Result<Arc<SynapseStore>, Status> {
        // Use entry API to ensure atomicity
        let store = self.stores.entry(namespace.to_string()).or_insert_with(|| {
            let s =
                SynapseStore::open(namespace, &self.storage_path).expect("Failed to open store");
            Arc::new(s)
        });

        Ok(store.value().clone())
    }
}

#[tonic::async_trait]
impl SemanticEngine for MySemanticEngine {
    async fn ingest_triples(
        &self,
        request: Request<IngestRequest>,
    ) -> Result<Response<IngestResponse>, Status> {
        // Auth check (Write permission)
        let token = get_token(&request);
        let req = request.into_inner();
        let namespace = if req.namespace.is_empty() {
            "default"
        } else {
            &req.namespace
        };

        if let Err(e) = self.auth.check(token.as_deref(), namespace, "write") {
            return Err(Status::permission_denied(e));
        }

        let store = self.get_store(namespace)?;

        // Log provenance for audit
        let timestamp = chrono::Utc::now().to_rfc3339();
        let triple_count = req.triples.len();
        let mut sources: Vec<String> = Vec::new();

        let triples: Vec<IngestTriple> = req
            .triples
            .into_iter()
            .map(|t| {
                // Capture provenance sources for logging
                if let Some(ref prov) = t.provenance {
                    if !prov.source.is_empty() && !sources.contains(&prov.source) {
                        sources.push(prov.source.clone());
                    }
                }
                IngestTriple {
                    subject: t.subject,
                    predicate: t.predicate,
                    object: t.object,
                    provenance: t.provenance.map(|p| crate::store::Provenance {
                        source: p.source,
                        timestamp: p.timestamp,
                        method: p.method,
                    }),
                }
            })
            .collect();

        match store.ingest_triples(triples).await {
            Ok((added, _)) => {
                // Log ingestion for audit trail
                eprintln!(
                    "INGEST [{timestamp}] namespace={namespace} triples={triple_count} added={added} sources={:?}",
                    sources
                );
                Ok(Response::new(IngestResponse {
                    nodes_added: added,
                    edges_added: added,
                }))
            }
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn ingest_file(
        &self,
        request: Request<IngestFileRequest>,
    ) -> Result<Response<IngestResponse>, Status> {
        // Auth check (Write permission) - previously missing? or just implicit?
        // Note: The original code didn't check auth for ingest_file!
        // Adding it now for consistency as we are touching auth.
        let token = get_token(&request);
        let req = request.into_inner();
        let namespace = if req.namespace.is_empty() {
            "default"
        } else {
            &req.namespace
        };

        if let Err(e) = self.auth.check(token.as_deref(), namespace, "write") {
            return Err(Status::permission_denied(e));
        }
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
        request: Request<NodeRequest>,
    ) -> Result<Response<NeighborResponse>, Status> {
        let token = get_token(&request);
        let req = request.into_inner();
        let namespace = if req.namespace.is_empty() {
            "default"
        } else {
            &req.namespace
        };

        if let Err(e) = self.auth.check(token.as_deref(), namespace, "read") {
            return Err(Status::permission_denied(e));
        }

        let store = self.get_store(namespace)?;

        let direction = if req.direction.is_empty() {
            "outgoing"
        } else {
            &req.direction
        };
        let edge_filter = if req.edge_filter.is_empty() {
            None
        } else {
            Some(req.edge_filter.as_str())
        };
        let node_type_filter = if req.node_type_filter.is_empty() {
            None
        } else {
            Some(req.node_type_filter.as_str())
        };
        let max_depth = if req.depth == 0 {
            1
        } else {
            req.depth as usize
        };
        let limit_per_layer = if req.limit_per_layer == 0 {
            usize::MAX
        } else {
            req.limit_per_layer as usize
        };

        let mut neighbors = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut current_frontier = Vec::new();

        // Start with the initial node
        if let Some(start_uri) = store.get_uri(req.node_id) {
            current_frontier.push(start_uri.clone());
            visited.insert(start_uri);
        }

        // BFS traversal up to max_depth
        for current_depth in 1..=max_depth {
            let mut next_frontier = Vec::new();
            let mut layer_count = 0;
            let base_score = 1.0 / current_depth as f32; // Path scoring: closer = higher

            for uri in &current_frontier {
                if layer_count >= limit_per_layer {
                    break;
                }

                // Query outgoing edges (URI as subject)
                if direction == "outgoing" || direction == "both" {
                    if let Ok(subj) = oxigraph::model::NamedNodeRef::new(uri) {
                        for quad in
                            store
                                .store
                                .quads_for_pattern(Some(subj.into()), None, None, None)
                        {
                            if layer_count >= limit_per_layer {
                                break;
                            }
                            if let Ok(q) = quad {
                                let pred = q.predicate.to_string();
                                // Apply edge filter if specified
                                if let Some(filter) = edge_filter {
                                    if !pred.contains(filter) {
                                        continue;
                                    }
                                }
                                let obj_term = q.object;
                                let obj_uri = obj_term.to_string();

                                // Node Type Filter Logic
                                if let Some(type_filter) = node_type_filter {
                                    let passed =
                                        if let oxigraph::model::Term::NamedNode(ref n) = obj_term {
                                            let rdf_type = oxigraph::model::NamedNodeRef::new(
                                                "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
                                            )
                                            .unwrap();
                                            if let Ok(target_type) =
                                                oxigraph::model::NamedNodeRef::new(type_filter)
                                            {
                                                store
                                                    .store
                                                    .quads_for_pattern(
                                                        Some(n.into()),
                                                        Some(rdf_type),
                                                        Some(target_type.into()),
                                                        None,
                                                    )
                                                    .next()
                                                    .is_some()
                                            } else {
                                                false
                                            }
                                        } else {
                                            false
                                        };
                                    if !passed {
                                        continue;
                                    }
                                }

                                let clean_uri = match &obj_term {
                                    oxigraph::model::Term::NamedNode(n) => n.as_str(),
                                    _ => &obj_uri,
                                };

                                // Always add to neighbors if not already in neighbors list to avoid duplicates there
                                // But we must allow revisiting nodes for graph expansion if we want to find paths?
                                // BFS typically avoids cycles by checking visited.

                                // NOTE: visited set prevents processing same node twice in BFS.
                                // If we reach a node that was already visited in a previous layer (or this layer), skip it.
                                if !visited.contains(&obj_uri) {
                                    visited.insert(obj_uri.clone());
                                    let obj_id = store.get_or_create_id(&obj_uri);

                                    let mut neighbor_score = base_score;
                                    if req.scoring_strategy == "degree" {
                                        let degree = store.get_degree(clean_uri);
                                        neighbor_score /= (degree as f32 + 1.0).ln().max(0.1);
                                    }

                                    neighbors.push(Neighbor {
                                        node_id: obj_id,
                                        edge_type: pred,
                                        uri: obj_uri.clone(), // This is the N-Triples formatted string for display
                                        direction: "outgoing".to_string(),
                                        depth: current_depth as u32,
                                        score: neighbor_score,
                                    });
                                    // Use clean_uri for next frontier to ensure we query with raw URI, not <uri>
                                    next_frontier.push(clean_uri.to_string());
                                    layer_count += 1;
                                }
                            }
                        }
                    }
                }

                // Query incoming edges (URI as object)
                if direction == "incoming" || direction == "both" {
                    if let Ok(obj) = oxigraph::model::NamedNodeRef::new(uri) {
                        for quad in
                            store
                                .store
                                .quads_for_pattern(None, None, Some(obj.into()), None)
                        {
                            if layer_count >= limit_per_layer {
                                break;
                            }
                            if let Ok(q) = quad {
                                let pred = q.predicate.to_string();
                                // Apply edge filter if specified
                                if let Some(filter) = edge_filter {
                                    if !pred.contains(filter) {
                                        continue;
                                    }
                                }
                                let subj_term = q.subject;
                                let subj_uri = subj_term.to_string();

                                // Node Type Filter Logic
                                if let Some(type_filter) = node_type_filter {
                                    let passed = if let oxigraph::model::Subject::NamedNode(ref n) =
                                        subj_term
                                    {
                                        let rdf_type = oxigraph::model::NamedNodeRef::new(
                                            "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
                                        )
                                        .unwrap();
                                        if let Ok(target_type) =
                                            oxigraph::model::NamedNodeRef::new(type_filter)
                                        {
                                            store
                                                .store
                                                .quads_for_pattern(
                                                    Some(n.into()),
                                                    Some(rdf_type),
                                                    Some(target_type.into()),
                                                    None,
                                                )
                                                .next()
                                                .is_some()
                                        } else {
                                            false
                                        }
                                    } else {
                                        false
                                    };
                                    if !passed {
                                        continue;
                                    }
                                }

                                let clean_uri = match &subj_term {
                                    oxigraph::model::Subject::NamedNode(n) => n.as_str(),
                                    _ => &subj_uri,
                                };

                                if !visited.contains(&subj_uri) {
                                    visited.insert(subj_uri.clone());
                                    let subj_id = store.get_or_create_id(&subj_uri);

                                    let mut neighbor_score = base_score;
                                    if req.scoring_strategy == "degree" {
                                        let degree = store.get_degree(clean_uri);
                                        // Penalize super nodes
                                        neighbor_score /= (degree as f32 + 1.0).ln().max(0.1);
                                    }

                                    neighbors.push(Neighbor {
                                        node_id: subj_id,
                                        edge_type: pred,
                                        uri: subj_uri.clone(),
                                        direction: "incoming".to_string(),
                                        depth: current_depth as u32,
                                        score: neighbor_score,
                                    });
                                    // Use clean_uri for next frontier
                                    next_frontier.push(clean_uri.to_string());
                                    layer_count += 1;
                                }
                            }
                        }
                    }
                }
            }

            current_frontier = next_frontier;
            if current_frontier.is_empty() {
                break;
            }
        }

        // Sort by score (highest first)
        neighbors.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(Response::new(NeighborResponse { neighbors }))
    }

    async fn search(
        &self,
        request: Request<SearchRequest>,
    ) -> Result<Response<SearchResponse>, Status> {
        let token = get_token(&request);
        let req = request.into_inner();
        let namespace = if req.namespace.is_empty() {
            "default"
        } else {
            &req.namespace
        };

        if let Err(e) = self.auth.check(token.as_deref(), namespace, "read") {
            return Err(Status::permission_denied(e));
        }

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
                Ok(Response::new(SearchResponse {
                    results: grpc_results,
                }))
            }
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn resolve_id(
        &self,
        request: Request<ResolveRequest>,
    ) -> Result<Response<ResolveResponse>, Status> {
        let token = get_token(&request);
        let req = request.into_inner();
        let namespace = if req.namespace.is_empty() {
            "default"
        } else {
            &req.namespace
        };

        if let Err(e) = self.auth.check(token.as_deref(), namespace, "read") {
            return Err(Status::permission_denied(e));
        }

        let store = self.get_store(namespace)?;

        let uri = store.ensure_uri(&req.content);

        // Look up the URI in our mapping
        let uri_to_id = store.uri_to_id.read().unwrap();
        if let Some(&node_id) = uri_to_id.get(&uri) {
            Ok(Response::new(ResolveResponse {
                node_id,
                found: true,
            }))
        } else {
            Ok(Response::new(ResolveResponse {
                node_id: 0,
                found: false,
            }))
        }
    }

    async fn get_all_triples(
        &self,
        request: Request<EmptyRequest>,
    ) -> Result<Response<TriplesResponse>, Status> {
        let token = get_token(&request);
        let req = request.into_inner();
        let namespace = if req.namespace.is_empty() {
            "default"
        } else {
            &req.namespace
        };

        if let Err(e) = self.auth.check(token.as_deref(), namespace, "read") {
            return Err(Status::permission_denied(e));
        }

        let store = self.get_store(namespace)?;

        let mut triples = Vec::new();

        for quad in store.store.iter().map(|q| q.unwrap()) {
            let s = quad.subject.to_string();
            let p = quad.predicate.to_string();
            let o = quad.object.to_string();

            // Clean up NTriples formatting (<uri> -> uri)
            let clean_s = if s.starts_with('<') && s.ends_with('>') {
                s[1..s.len() - 1].to_string()
            } else {
                s
            };
            let clean_p = if p.starts_with('<') && p.ends_with('>') {
                p[1..p.len() - 1].to_string()
            } else {
                p
            };
            let clean_o = if o.starts_with('<') && o.ends_with('>') {
                o[1..o.len() - 1].to_string()
            } else {
                o
            };

            triples.push(Triple {
                subject: clean_s,
                predicate: clean_p,
                object: clean_o,
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
        let token = get_token(&request);
        let req = request.into_inner();
        let namespace = if req.namespace.is_empty() {
            "default"
        } else {
            &req.namespace
        };

        if let Err(e) = self.auth.check(token.as_deref(), namespace, "read") {
            return Err(Status::permission_denied(e));
        }

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
        let token = get_token(&request);
        let req = request.into_inner();
        let namespace = if req.namespace.is_empty() {
            "default"
        } else {
            &req.namespace
        };

        if let Err(e) = self.auth.check(token.as_deref(), namespace, "delete") {
            return Err(Status::permission_denied(e));
        }

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
        let token = get_token(&request);
        let req = request.into_inner();
        let namespace = if req.namespace.is_empty() {
            "default"
        } else {
            &req.namespace
        };

        if let Err(e) = self.auth.check(token.as_deref(), namespace, "read") {
            return Err(Status::permission_denied(e));
        }

        let store = self.get_store(namespace)?;

        let vector_k = req.vector_k as usize;
        let graph_depth = req.graph_depth;

        let results = match SearchMode::try_from(req.mode) {
            Ok(SearchMode::VectorOnly) | Ok(SearchMode::Hybrid) => store
                .hybrid_search(&req.query, vector_k, graph_depth)
                .await
                .map_err(|e| Status::internal(format!("Hybrid search failed: {}", e)))?,
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

        Ok(Response::new(SearchResponse {
            results: grpc_results,
        }))
    }

    async fn apply_reasoning(
        &self,
        request: Request<ReasoningRequest>,
    ) -> Result<Response<ReasoningResponse>, Status> {
        // Auth check (Reason permission)
        let token = get_token(&request);
        let req = request.into_inner();
        let namespace = if req.namespace.is_empty() {
            "default"
        } else {
            &req.namespace
        };

        if let Err(e) = self.auth.check(token.as_deref(), namespace, "reason") {
            return Err(Status::permission_denied(e));
        }

        let store = self.get_store(namespace)?;

        let strategy = match ReasoningStrategy::try_from(req.strategy) {
            Ok(ReasoningStrategy::Rdfs) => InternalStrategy::RDFS,
            Ok(ReasoningStrategy::Owlrl) => InternalStrategy::OWLRL,
            _ => InternalStrategy::None,
        };
        let strategy_name = format!("{:?}", strategy);

        let reasoner = SynapseReasoner::new(strategy);
        let start_triples = store.store.len().unwrap_or(0);

        let response = if req.materialize {
            match reasoner.materialize(&store.store) {
                Ok(count) => Ok(Response::new(ReasoningResponse {
                    success: true,
                    triples_inferred: count as u32,
                    message: format!(
                        "Materialized {} triples in namespace '{}'",
                        count, namespace
                    ),
                })),
                Err(e) => Err(Status::internal(e.to_string())),
            }
        } else {
            match reasoner.apply(&store.store) {
                Ok(triples) => Ok(Response::new(ReasoningResponse {
                    success: true,
                    triples_inferred: triples.len() as u32,
                    message: format!(
                        "Found {} inferred triples in namespace '{}'",
                        triples.len(),
                        namespace
                    ),
                })),
                Err(e) => Err(Status::internal(e.to_string())),
            }
        };

        // Audit Log
        if let Ok(ref res) = response {
            let inferred = res.get_ref().triples_inferred as usize;
            self.audit.log(
                namespace,
                &strategy_name,
                start_triples,
                inferred,
                0, // Duplicates skipped not easily tracked here without changing reasoner return signature
                vec![], // Sample inferences
            );
        }

        response
    }
}

pub async fn run_mcp_stdio(
    engine: Arc<MySemanticEngine>,
) -> Result<(), Box<dyn std::error::Error>> {
    let server = crate::mcp_stdio::McpStdioServer::new(engine);
    server.run().await
}
