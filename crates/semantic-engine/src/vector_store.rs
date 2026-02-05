use anyhow::Result;
use hnsw::Hnsw;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::path::PathBuf;
use rand_pcg::Pcg64;

const HUGGINGFACE_API_URL: &str = "https://router.huggingface.co/hf-inference/models";
const DEFAULT_MODEL: &str = "sentence-transformers/all-MiniLM-L6-v2"; // 384 dims, fast
const DEFAULT_DIMENSIONS: usize = 384;

/// Euclidean distance metric for HNSW
#[derive(Default, Clone)]
pub struct Euclidian;

impl space::Metric<Vec<f32>> for Euclidian {
    type Unit = u64;
    fn distance(&self, a: &Vec<f32>, b: &Vec<f32>) -> u64 {
        let len = a.len().min(b.len());
        let mut dist_sq = 0.0;
        for i in 0..len {
            let diff = a[i] - b[i];
            dist_sq += diff * diff;
        }
        // Floating point to bits for ordered comparison as per space v0.17 recommendations
        dist_sq.sqrt().to_bits() as u64
    }
}

/// Persisted vector data
#[derive(Serialize, Deserialize, Default)]
struct VectorData {
    entries: Vec<VectorEntry>,
}

#[derive(Serialize, Deserialize, Clone)]
struct VectorEntry {
    uri: String,
    embedding: Vec<f32>,
}

/// Vector store using HuggingFace Inference API for embeddings
pub struct VectorStore {
    /// HNSW index for fast approximate nearest neighbor search
    index: Arc<RwLock<Hnsw<Euclidian, Vec<f32>, Pcg64, 16, 32>>>,
    /// Mapping from node ID to URI
    id_to_uri: Arc<RwLock<HashMap<usize, String>>>,
    /// Mapping from URI to node ID
    uri_to_id: Arc<RwLock<HashMap<String, usize>>>,
    /// Storage path for persistence
    storage_path: Option<PathBuf>,
    /// HTTP client for HuggingFace API
    client: Client,
    /// HuggingFace API token (optional, for rate limits)
    api_token: Option<String>,
    /// Model name
    model: String,
    /// Vector dimensions
    dimensions: usize,
    /// Stored embeddings for persistence
    embeddings: Arc<RwLock<Vec<VectorEntry>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub uri: String,
    pub score: f32,
    pub content: String,
}

#[derive(Serialize)]
struct EmbeddingRequest {
    inputs: String,
}

impl VectorStore {
    /// Create a new vector store for a namespace
    pub fn new(namespace: &str) -> Result<Self> {
        // Try to get storage path from environment
        let storage_path = std::env::var("GRAPH_STORAGE_PATH")
            .ok()
            .map(|p| PathBuf::from(p).join(namespace));

        // Get dimensions from env or default
        let dimensions = std::env::var("VECTOR_DIMENSIONS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_DIMENSIONS);
        
        // Create HNSW index with Euclidian metric
        let mut index = Hnsw::new(Euclidian);
        let mut id_to_uri = HashMap::new();
        let mut uri_to_id = HashMap::new();
        let mut embeddings = Vec::new();

        // Try to load persisted vectors
        if let Some(ref path) = storage_path {
            let vectors_path = path.join("vectors.json");
            if vectors_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&vectors_path) {
                    if let Ok(data) = serde_json::from_str::<VectorData>(&content) {
                        let mut searcher = hnsw::Searcher::default();
                        for entry in data.entries {
                            if entry.embedding.len() == dimensions {
                                let id = index.insert(entry.embedding.clone(), &mut searcher);
                                id_to_uri.insert(id, entry.uri.clone());
                                uri_to_id.insert(entry.uri.clone(), id);
                                embeddings.push(entry);
                            }
                        }
                        eprintln!("Loaded {} vectors from disk (dim={})", embeddings.len(), dimensions);
                    }
                }
            }
        }

        // Get API token from environment (optional)
        let api_token = std::env::var("HUGGINGFACE_API_TOKEN").ok();

        Ok(Self {
            index: Arc::new(RwLock::new(index)),
            id_to_uri: Arc::new(RwLock::new(id_to_uri)),
            uri_to_id: Arc::new(RwLock::new(uri_to_id)),
            storage_path,
            client: Client::new(),
            api_token,
            model: DEFAULT_MODEL.to_string(),
            dimensions,
            embeddings: Arc::new(RwLock::new(embeddings)),
        })
    }

    /// Save vectors to disk
    fn save_vectors(&self) -> Result<()> {
        if let Some(ref path) = self.storage_path {
            std::fs::create_dir_all(path)?;
            let data = VectorData {
                entries: self.embeddings.read().unwrap().clone(),
            };
            let content = serde_json::to_string(&data)?;
            std::fs::write(path.join("vectors.json"), content)?;
        }
        Ok(())
    }

    /// Generate embedding for a text using HuggingFace Inference API
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let url = format!("{}/{}/pipeline/feature-extraction", HUGGINGFACE_API_URL, self.model);
        
        let mut request = self.client
            .post(&url)
            .json(&EmbeddingRequest {
                inputs: text.to_string(),
            });

        // Add auth token if available
        if let Some(ref token) = self.api_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("HuggingFace API error: {}", error_text);
        }

        // Response is a Vec<f32> directly
        let embedding_vec: Vec<f32> = response.json().await?;
        
        if embedding_vec.len() != self.dimensions {
            anyhow::bail!("Expected {} dimensions, got {}", self.dimensions, embedding_vec.len());
        }
        
        Ok(embedding_vec)
    }

    /// Add a URI with its text content to the index
    pub async fn add(&self, uri: &str, content: &str) -> Result<usize> {
        // Check if URI already exists
        {
            let uri_map = self.uri_to_id.read().unwrap();
            if let Some(&id) = uri_map.get(uri) {
                return Ok(id);
            }
        }

        // Generate embedding via HuggingFace API
        let embedding = self.embed(content).await?;
        
        // Add to HNSW index
        let mut searcher = hnsw::Searcher::default();
        let id = {
            let mut index = self.index.write().unwrap();
            index.insert(embedding.clone(), &mut searcher)
        };

        // Update mappings
        {
            let mut uri_map = self.uri_to_id.write().unwrap();
            let mut id_map = self.id_to_uri.write().unwrap();
            uri_map.insert(uri.to_string(), id);
            id_map.insert(id, uri.to_string());
        }

        // Persist embedding for recovery
        {
            let mut embs = self.embeddings.write().unwrap();
            embs.push(VectorEntry {
                uri: uri.to_string(),
                embedding: embedding,
            });
        }
        let _ = self.save_vectors(); // Best effort persistence

        Ok(id)
    }

    /// Search for similar vectors
    pub async fn search(&self, query: &str, k: usize) -> Result<Vec<SearchResult>> {
        // Generate query embedding via HuggingFace API
        let query_embedding = self.embed(query).await?;

        // Search HNSW index
        let mut searcher = hnsw::Searcher::default();
        let mut neighbors = vec![space::Neighbor { index: 0, distance: !0 }; k];
        
        let found_neighbors = {
            let index = self.index.read().unwrap();
            index.nearest(&query_embedding, 50, &mut searcher, &mut neighbors)
        };

        // Convert to results
        let id_map = self.id_to_uri.read().unwrap();
        let results: Vec<SearchResult> = found_neighbors
            .iter()
            .filter_map(|neighbor| {
                id_map.get(&neighbor.index).map(|uri| {
                    // Convert back from bits to f32
                    let score_f32 = f32::from_bits(neighbor.distance as u32);
                    SearchResult {
                        uri: uri.clone(),
                        score: 1.0 / (1.0 + score_f32), 
                        content: uri.clone(), 
                    }
                })
            })
            .collect();

        Ok(results)
    }

    pub fn get_uri(&self, id: usize) -> Option<String> {
        self.id_to_uri.read().unwrap().get(&id).cloned()
    }

    pub fn get_id(&self, uri: &str) -> Option<usize> {
        self.uri_to_id.read().unwrap().get(uri).copied()
    }

    pub fn len(&self) -> usize {
        self.uri_to_id.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Compaction: rebuild index from stored embeddings, removing stale entries
    pub fn compact(&self) -> Result<usize> {
        let embeddings = self.embeddings.read().unwrap();
        let current_uris: std::collections::HashSet<_> = self.uri_to_id.read().unwrap().keys().cloned().collect();
        
        // Safeguard: If uri_to_id is empty, avoid compaction unless embeddings is also empty
        // to prevent accidental deletion if mappings were lost.
        if current_uris.is_empty() && !embeddings.is_empty() {
            // We return 0 and skip compaction to be safe
            return Ok(0);
        }

        // Filter to only current URIs
        let active_entries: Vec<_> = embeddings.iter()
            .filter(|e| current_uris.contains(&e.uri))
            .cloned()
            .collect();

        let removed = embeddings.len() - active_entries.len();
        
        if removed == 0 {
            return Ok(0);
        }

        // Rebuild index
        let mut new_index = hnsw::Hnsw::new(Euclidian);
        let mut new_id_to_uri = std::collections::HashMap::new();
        let mut new_uri_to_id = std::collections::HashMap::new();
        let mut searcher = hnsw::Searcher::default();

        for entry in &active_entries {
            if entry.embedding.len() == self.dimensions {
                let id = new_index.insert(entry.embedding.clone(), &mut searcher);
                new_id_to_uri.insert(id, entry.uri.clone());
                new_uri_to_id.insert(entry.uri.clone(), id);
            }
        }

        // Swap in new index
        *self.index.write().unwrap() = new_index;
        *self.id_to_uri.write().unwrap() = new_id_to_uri;
        *self.uri_to_id.write().unwrap() = new_uri_to_id;
        
        // Update embeddings (drop takes write lock)
        drop(embeddings);
        *self.embeddings.write().unwrap() = active_entries;
        
        let _ = self.save_vectors();
        
        Ok(removed)
    }

    /// Remove a URI from the vector store
    pub fn remove(&self, uri: &str) -> bool {
        let mut uri_map = self.uri_to_id.write().unwrap();
        let mut id_map = self.id_to_uri.write().unwrap();

        if let Some(id) = uri_map.remove(uri) {
            id_map.remove(&id);
            // Note: actual index entry remains until compaction
            true
        } else {
            false
        }
    }

    /// Get storage stats
    pub fn stats(&self) -> (usize, usize, usize) {
        let embeddings_count = self.embeddings.read().unwrap().len();
        let active_count = self.uri_to_id.read().unwrap().len();
        let stale_count = embeddings_count.saturating_sub(active_count);
        (active_count, stale_count, embeddings_count)
    }
}
