use crate::persistence::{load_bincode, save_bincode};
use crate::vector_store::VectorStore;
use anyhow::Result;
use oxigraph::model::*;
use oxigraph::store::Store;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use uuid::Uuid;

const DEFAULT_MAPPING_SAVE_THRESHOLD: usize = 1000;

/// Persisted URI mappings
#[derive(Serialize, Deserialize, Default)]
struct UriMappings {
    uri_to_id: HashMap<String, u32>,
    next_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Provenance {
    pub source: String,
    pub timestamp: String,
    pub method: String,
}

pub struct IngestTriple {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub provenance: Option<Provenance>,
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
    // Persistence state
    dirty_count: AtomicUsize,
    save_threshold: usize,
}

impl SynapseStore {
    pub fn open(namespace: &str, storage_path: &str) -> Result<Self> {
        let path = PathBuf::from(storage_path).join(namespace);
        std::fs::create_dir_all(&path)?;

        #[cfg(feature = "rocksdb")]
        let store = Store::open(&path)?;

        #[cfg(not(feature = "rocksdb"))]
        let store = {
            let s = Store::new()?;
            let graph_path = path.join("graph.nq");
            if graph_path.exists() {
                let file = std::fs::File::open(&graph_path)?;
                let reader = std::io::BufReader::new(file);
                s.load_from_reader(oxigraph::io::RdfFormat::NQuads, reader)?;
                eprintln!("Loaded in-memory graph from {}", graph_path.display());
            }
            s
        };

        // Load persisted URI mappings if they exist
        let mappings_path_bin = path.join("uri_mappings.bin");
        let mappings_path_json = path.join("uri_mappings.json");

        let (uri_to_id, id_to_uri, next_id) = if mappings_path_bin.exists() {
            let mappings: UriMappings = load_bincode(&mappings_path_bin)?;
            let id_to_uri: HashMap<u32, String> = mappings
                .uri_to_id
                .iter()
                .map(|(uri, &id)| (id, uri.clone()))
                .collect();
            (mappings.uri_to_id, id_to_uri, mappings.next_id)
        } else if mappings_path_json.exists() {
            let content = std::fs::read_to_string(&mappings_path_json)?;
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
        let vector_store = match VectorStore::new(namespace) {
            Ok(vs) => Some(Arc::new(vs)),
            Err(e) => {
                eprintln!("WARNING: Failed to initialize vector store for namespace '{}': {}", namespace, e);
                None
            }
        };

        Ok(Self {
            store,
            namespace: namespace.to_string(),
            storage_path: path,
            id_to_uri: RwLock::new(id_to_uri),
            uri_to_id: RwLock::new(uri_to_id),
            next_id: std::sync::atomic::AtomicU32::new(next_id),
            vector_store,
            dirty_count: AtomicUsize::new(0),
            save_threshold: DEFAULT_MAPPING_SAVE_THRESHOLD,
        })
    }

    /// Save URI mappings to disk
    fn save_mappings(&self) -> Result<()> {
        let mappings = UriMappings {
            uri_to_id: self.uri_to_id.read().unwrap().clone(),
            next_id: self.next_id.load(std::sync::atomic::Ordering::Relaxed),
        };
        // Capture the count before saving? No, we just care that we saved the current state.
        // But if new items are added during save, the dirty count will increment.
        // We need to subtract what we think we saved.
        // Since we save the *entire* map, we effectively save *all* dirty items up to that point.
        // So we can read the dirty count, save, then subtract.
        let current_dirty = self.dirty_count.load(Ordering::Relaxed);

        save_bincode(&self.storage_path.join("uri_mappings.bin"), &mappings)?;

        if current_dirty > 0 {
            let _ = self.dirty_count.fetch_sub(current_dirty, Ordering::Relaxed);
        }
        Ok(())
    }

    /// Force save all data to disk
    pub fn flush(&self) -> Result<()> {
        self.save_mappings()?;
        if let Some(ref vs) = self.vector_store {
            vs.flush()?;
        }

        #[cfg(not(feature = "rocksdb"))]
        {
            let graph_path = self.storage_path.join("graph.nq");
            // Atomic write pattern: write to tmp, then rename
            let tmp_path = self.storage_path.join("graph.nq.tmp");
            let file = std::fs::File::create(&tmp_path)?;
            let writer = std::io::BufWriter::new(file);
            self.store.dump_to_writer(oxigraph::io::RdfFormat::NQuads, writer)?;
            std::fs::rename(tmp_path, graph_path)?;
            eprintln!("Persisted in-memory graph to disk.");
        }

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

        drop(uri_map);
        drop(id_map);

        // Check if we need to auto-save mappings
        let count = self.dirty_count.fetch_add(1, Ordering::Relaxed);
        if count + 1 >= self.save_threshold {
            let _ = self.save_mappings();
        }

        id
    }

    pub fn get_uri(&self, id: u32) -> Option<String> {
        self.id_to_uri.read().unwrap().get(&id).cloned()
    }

    pub async fn ingest_triples(&self, triples: Vec<IngestTriple>) -> Result<(u32, u32)> {
        let mut added = 0;

        // Group by provenance to optimize batch insertion into named graphs
        let mut batches: HashMap<Option<Provenance>, Vec<(String, String, String)>> =
            HashMap::new();

        for t in triples {
            batches
                .entry(t.provenance)
                .or_default()
                .push((t.subject, t.predicate, t.object));
        }

        for (prov, batch_triples) in batches {
            let graph_name = if let Some(p) = &prov {
                let uuid = Uuid::new_v4();
                let uri = format!("urn:batch:{}", uuid);

                let batch_node = NamedNode::new_unchecked(&uri);
                let p_derived =
                    NamedNode::new_unchecked("http://www.w3.org/ns/prov#wasDerivedFrom");
                let p_time = NamedNode::new_unchecked("http://www.w3.org/ns/prov#generatedAtTime");
                let p_method = NamedNode::new_unchecked("http://www.w3.org/ns/prov#wasGeneratedBy");

                let o_source = Literal::new_simple_literal(&p.source);
                let o_time = Literal::new_simple_literal(&p.timestamp);
                let o_method = Literal::new_simple_literal(&p.method);

                self.store.insert(&Quad::new(
                    batch_node.clone(),
                    p_derived,
                    o_source,
                    GraphName::DefaultGraph,
                ))?;
                self.store.insert(&Quad::new(
                    batch_node.clone(),
                    p_time,
                    o_time,
                    GraphName::DefaultGraph,
                ))?;
                self.store.insert(&Quad::new(
                    batch_node.clone(),
                    p_method,
                    o_method,
                    GraphName::DefaultGraph,
                ))?;

                // If source is "mcp", put triples in default graph for easier querying
                if p.source == "mcp" {
                    GraphName::DefaultGraph
                } else {
                    GraphName::NamedNode(batch_node)
                }
            } else {
                GraphName::DefaultGraph
            };

            for (s, p, o) in batch_triples {
                let subject_uri = self.ensure_uri(&s);
                let predicate_uri = self.ensure_uri(&p);

                let (object_term, object_key_str) = if o.starts_with('"') && o.ends_with('"') && o.len() >= 2 {
                    let literal_val = &o[1..o.len() - 1];
                    (Term::Literal(Literal::new_simple_literal(literal_val)), literal_val.to_string())
                } else {
                    let uri = self.ensure_uri(&o);
                    (Term::NamedNode(NamedNode::new_unchecked(&uri)), uri)
                };

                // Register URIs in the ID mapping (for gRPC compatibility)
                self.get_or_create_id(&subject_uri);
                self.get_or_create_id(&predicate_uri);
                self.get_or_create_id(&object_key_str);

                let subject = Subject::NamedNode(NamedNode::new_unchecked(&subject_uri));
                let predicate = NamedNode::new_unchecked(&predicate_uri);
                let object = object_term;

                let quad = Quad::new(subject, predicate, object, graph_name.clone());
                let inserted = self.store.insert(&quad)?;
                
                // Also index in vector store if available
                if let Some(ref vs) = self.vector_store {
                    // We check if it's already in the vector store by key
                    let key = format!("{}|{}|{}", subject_uri, predicate_uri, object_key_str);
                    if vs.get_id(&key).is_none() {
                        // Create searchable content from triple
                        let content = format!("{} {} {}", s, p, o);
                        // Pass metadata including the subject URI for graph expansion later
                        let metadata = serde_json::json!({
                            "uri": subject_uri,
                            "predicate": predicate_uri,
                            "object": object_key_str,
                            "type": "triple"
                        });

                        if let Err(e) = vs.add(&key, &content, metadata).await {
                            // If we just inserted it into the graph but vector failed, 
                            // we technically have an inconsistency, but for now we just log.
                            eprintln!("Vector store insertion failed for {}: {}", key, e);
                        }
                    }
                }

                if inserted {
                    added += 1;
                }
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
                // Use the URI from metadata/result (which maps to Subject URI for triples)
                let uri = result.uri.clone();
                results.push((uri.clone(), result.score));

                // Step 2: Graph expansion (if depth > 0)
                if graph_depth > 0 {
                    let expanded = self.expand_graph(&uri, graph_depth)?;
                    for expanded_uri in expanded {
                        // Add with slightly lower score
                        results.push((expanded_uri, result.score * 0.8));
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
            for q in self
                .store
                .quads_for_pattern(Some(subj.into()), None, None, None)
                .flatten()
            {
                expanded.push(q.object.to_string());

                // Recursive expansion (simplified, depth-1)
                if depth > 1 {
                    let nested = self.expand_graph(&q.object.to_string(), depth - 1)?;
                    expanded.extend(nested);
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

    pub fn get_degree(&self, uri: &str) -> usize {
        let node = NamedNodeRef::new(uri).ok();
        if let Some(n) = node {
            let outgoing = self
                .store
                .quads_for_pattern(Some(n.into()), None, None, None)
                .count();
            let incoming = self
                .store
                .quads_for_pattern(None, None, Some(n.into()), None)
                .count();
            outgoing + incoming
        } else {
            0
        }
    }

    pub fn ensure_uri(&self, s: &str) -> String {
        let clean = s.trim_start_matches('<').trim_end_matches('>');
        if clean.starts_with("http") || clean.starts_with("urn:") {
            clean.to_string()
        } else {
            format!("http://synapse.os/{}", clean)
        }
    }
}
