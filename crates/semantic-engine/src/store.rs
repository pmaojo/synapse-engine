use crate::vector_store::VectorStore;
use anyhow::Result;
use oxigraph::model::*;
use oxigraph::store::Store;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Persisted URI mappings
#[derive(Serialize, Deserialize, Default)]
struct UriMappings {
    uri_to_id: HashMap<String, u32>,
    next_id: u32,
}

pub struct SynapseStore {
    pub store: Store,
    pub namespace: String,
    pub storage_path: PathBuf,
    // Mapping for gRPC compatibility (ID <-> URI)
    pub id_to_uri: RwLock<HashMap<u32, String>>,
    pub uri_to_id: RwLock<HashMap<String, u32>>,
    pub next_id: std::sync::atomic::AtomicU32,
    // Vector store for hybrid search
    pub vector_store: Option<Arc<VectorStore>>,
}

impl SynapseStore {
    pub fn open(namespace: &str, storage_path: &str) -> Result<Self> {
        let path = PathBuf::from(storage_path).join(namespace);
        std::fs::create_dir_all(&path)?;
        let store = Store::open(&path)?;

        // Load persisted URI mappings if they exist
        let mappings_path = path.join("uri_mappings.json");
        let (uri_to_id, id_to_uri, next_id) = if mappings_path.exists() {
            let content = std::fs::read_to_string(&mappings_path)?;
            let mappings: UriMappings = serde_json::from_str(&content)?;
            let id_to_uri: HashMap<u32, String> = mappings
                .uri_to_id
                .iter()
                .map(|(uri, &id)| (id, uri.clone()))
                .collect();
            (mappings.uri_to_id, id_to_uri, mappings.next_id)
        } else {
            (HashMap::new(), HashMap::new(), 1)
        };

        // Initialize vector store (optional, can fail gracefully)
        let vector_store = VectorStore::new(namespace).ok().map(Arc::new);

        Ok(Self {
            store,
            namespace: namespace.to_string(),
            storage_path: path,
            id_to_uri: RwLock::new(id_to_uri),
            uri_to_id: RwLock::new(uri_to_id),
            next_id: std::sync::atomic::AtomicU32::new(next_id),
            vector_store,
        })
    }

    /// Save URI mappings to disk
    fn save_mappings(&self, mappings: UriMappings) -> Result<()> {
        let content = serde_json::to_string_pretty(&mappings)?;
        std::fs::write(self.storage_path.join("uri_mappings.json"), content)?;
        Ok(())
    }

    pub fn get_or_create_id(&self, uri: &str) -> u32 {
        {
            let map = self.uri_to_id.read().unwrap();
            if let Some(&id) = map.get(uri) {
                return id;
            }
        }

        let mut uri_map = self.uri_to_id.write().unwrap();
        let mut id_map = self.id_to_uri.write().unwrap();

        if let Some(&id) = uri_map.get(uri) {
            return id;
        }

        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        uri_map.insert(uri.to_string(), id);
        id_map.insert(id, uri.to_string());

        // Prepare mappings for persistence
        let mappings = UriMappings {
            uri_to_id: uri_map.clone(),
            next_id: self.next_id.load(std::sync::atomic::Ordering::Relaxed),
        };

        // Persist mappings (best effort, don't block on error)
        drop(uri_map);
        drop(id_map);
        let _ = self.save_mappings(mappings);

        id
    }

    pub fn get_uri(&self, id: u32) -> Option<String> {
        self.id_to_uri.read().unwrap().get(&id).cloned()
    }

    pub async fn ingest_triples(
        &self,
        triples: Vec<(String, String, String)>,
    ) -> Result<(u32, u32)> {
        let mut added = 0;
        let mut vector_items = Vec::new();

        for (s, p, o) in triples {
            let subject_uri = self.ensure_uri(&s);
            let predicate_uri = self.ensure_uri(&p);
            let object_uri = self.ensure_uri(&o);

            let subject = Subject::NamedNode(NamedNode::new_unchecked(&subject_uri));
            let predicate = NamedNode::new_unchecked(&predicate_uri);
            let object = Term::NamedNode(NamedNode::new_unchecked(&object_uri));

            let quad = Quad::new(subject, predicate, object, GraphName::DefaultGraph);
            if self.store.insert(&quad)? {
                // Also index in vector store if available
                if let Some(ref vs) = self.vector_store {
                    // Create searchable content from triple
                    let content = format!("{} {} {}", s, p, o);
                    if let Err(e) = vs.add(&subject_uri, &content).await {
                        // Rollback graph insertion to ensure consistency
                        self.store.remove(&quad)?;
                        return Err(anyhow::anyhow!(
                            "Vector store insertion failed, rolled back graph change: {}",
                            e
                        ));
                    }
                }
                added += 1;
            }
        }

        Ok((added, 0))
    }

    /// Hybrid search: vector similarity + graph expansion
    pub async fn hybrid_search(
        &self,
        query: &str,
        vector_k: usize,
        graph_depth: u32,
    ) -> Result<Vec<(String, f32)>> {
        let mut results = Vec::new();

        // Step 1: Vector search
        if let Some(ref vs) = self.vector_store {
            let vector_results = vs.search(query, vector_k).await?;

            for result in vector_results {
                results.push((result.uri.clone(), result.score));

                // Step 2: Graph expansion (if depth > 0)
                if graph_depth > 0 {
                    let expanded = self.expand_graph(&result.uri, graph_depth)?;
                    for uri in expanded {
                        // Add with slightly lower score
                        results.push((uri, result.score * 0.8));
                    }
                }
            }
        }

        // Remove duplicates and sort by score
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        results.dedup_by(|a, b| a.0 == b.0);

        Ok(results)
    }

    /// Expand graph from a starting URI
    fn expand_graph(&self, start_uri: &str, depth: u32) -> Result<Vec<String>> {
        let mut expanded = Vec::new();

        if depth == 0 {
            return Ok(expanded);
        }

        // Query for all triples where start_uri is subject or object
        let subject = NamedNodeRef::new(start_uri).ok();

        if let Some(subj) = subject {
            for quad in self
                .store
                .quads_for_pattern(Some(subj.into()), None, None, None)
            {
                if let Ok(q) = quad {
                    expanded.push(q.object.to_string());

                    // Recursive expansion (simplified, depth-1)
                    if depth > 1 {
                        let nested = self.expand_graph(&q.object.to_string(), depth - 1)?;
                        expanded.extend(nested);
                    }
                }
            }
        }

        Ok(expanded)
    }

    pub fn query_sparql(&self, query: &str) -> Result<String> {
        use oxigraph::sparql::QueryResults;

        let results = self.store.query(query)?;

        match results {
            QueryResults::Solutions(solutions) => {
                let mut results_array = Vec::new();
                for solution in solutions {
                    let sol = solution?;
                    let mut mapping = serde_json::Map::new();
                    for (variable, value) in sol.iter() {
                        mapping.insert(
                            variable.to_string(),
                            serde_json::to_value(value.to_string()).unwrap(),
                        );
                    }
                    results_array.push(serde_json::Value::Object(mapping));
                }
                Ok(serde_json::to_string(&results_array)?)
            }
            _ => Ok("[]".to_string()),
        }
    }

    fn ensure_uri(&self, s: &str) -> String {
        if s.starts_with("http") {
            s.to_string()
        } else {
            format!("http://synapse.os/{}", s)
        }
    }
}
