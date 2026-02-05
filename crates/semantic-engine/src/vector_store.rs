use anyhow::Result;
use hnsw::Hnsw;
use rand_pcg::Pcg64;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
const HUGGINGFACE_API_URL: &str = "https://router.huggingface.co/hf-inference/models";
const DEFAULT_MODEL: &str = "sentence-transformers/all-MiniLM-L6-v2"; // 384 dims, fast

/// Euclidean distance metric for HNSW
#[derive(Default, Clone)]
pub struct Euclidian;

impl space::Metric<[f32; 384]> for Euclidian {
    type Unit = u32;
    fn distance(&self, a: &[f32; 384], b: &[f32; 384]) -> u32 {
        let mut dist_sq = 0.0;
        for i in 0..384 {
            let diff = a[i] - b[i];
            dist_sq += diff * diff;
        }
        // Use fixed-point arithmetic to avoid bitwise comparison issues
        (dist_sq.sqrt() * 1_000_000.0) as u32
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
    index: Arc<RwLock<Hnsw<Euclidian, [f32; 384], Pcg64, 16, 32>>>,
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
    /// Stored embeddings for persistence
    embeddings: Arc<RwLock<Vec<VectorEntry>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub uri: String,
    pub score: f32,
    pub content: String,
}

impl VectorStore {
    /// Create a new vector store for a namespace
    pub fn new(namespace: &str) -> Result<Self> {
        // Try to get storage path from environment
        let storage_path = std::env::var("GRAPH_STORAGE_PATH")
            .ok()
            .map(|p| PathBuf::from(p).join(namespace));

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
                            if entry.embedding.len() == 384 {
                                let mut emb = [0.0f32; 384];
                                emb.copy_from_slice(&entry.embedding);
                                let id = index.insert(emb, &mut searcher);
                                id_to_uri.insert(id, entry.uri.clone());
                                uri_to_id.insert(entry.uri.clone(), id);
                                embeddings.push(entry);
                            }
                        }
                        eprintln!("Loaded {} vectors from disk", embeddings.len());
                    }
                }
            }
        }

        // Get API token from environment (optional)
        let api_token = std::env::var("HUGGINGFACE_API_TOKEN").ok();

        // Configured client with timeout
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());

        Ok(Self {
            index: Arc::new(RwLock::new(index)),
            id_to_uri: Arc::new(RwLock::new(id_to_uri)),
            uri_to_id: Arc::new(RwLock::new(uri_to_id)),
            storage_path,
            client,
            api_token,
            model: DEFAULT_MODEL.to_string(),
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
    pub async fn embed(&self, text: &str) -> Result<[f32; 384]> {
        let embeddings = self.embed_batch(vec![text.to_string()]).await?;
        Ok(embeddings[0])
    }

    /// Generate embeddings for multiple texts using HuggingFace Inference API
    pub async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<[f32; 384]>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let url = format!(
            "{}/{}/pipeline/feature-extraction",
            HUGGINGFACE_API_URL, self.model
        );

        // HuggingFace accepts array of strings for inputs
        let mut request = self.client.post(&url).json(&serde_json::json!({
            "inputs": texts,
        }));

        // Add auth token if available
        if let Some(ref token) = self.api_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("HuggingFace API error: {}", error_text);
        }

        let response_json: serde_json::Value = response.json().await?;

        let mut results = Vec::new();

        if let Some(arr) = response_json.as_array() {
            for item in arr {
                let vec: Vec<f32> = serde_json::from_value(item.clone())
                    .map_err(|e| anyhow::anyhow!("Failed to parse embedding: {}", e))?;

                if vec.len() != 384 {
                    anyhow::bail!("Expected 384 dimensions, got {}", vec.len());
                }

                let mut embedding = [0.0f32; 384];
                embedding.copy_from_slice(&vec[0..384]);
                results.push(embedding);
            }
        } else {
            // Handle case where we sent 1 text and got single flat array [0.1, ...]
            if texts.len() == 1 {
                if let Ok(vec) = serde_json::from_value::<Vec<f32>>(response_json) {
                    if vec.len() == 384 {
                        let mut embedding = [0.0f32; 384];
                        embedding.copy_from_slice(&vec[0..384]);
                        results.push(embedding);
                    }
                }
            }
        }

        if results.len() != texts.len() {
            anyhow::bail!("Expected {} embeddings, got {}", texts.len(), results.len());
        }

        Ok(results)
    }

    /// Add a URI with its text content to the index
    pub async fn add(&self, uri: &str, content: &str) -> Result<usize> {
        self.add_batch(vec![(uri.to_string(), content.to_string())])
            .await
            .map(|v| v[0])
    }

    /// Add multiple URIs with their text content to the index
    pub async fn add_batch(&self, items: Vec<(String, String)>) -> Result<Vec<usize>> {
        // Filter out existing URIs
        let mut new_items = Vec::new();
        let mut result_ids = vec![0; items.len()];
        let mut new_indices = Vec::new(); // Map index in new_items to index in items

        {
            let uri_map = self.uri_to_id.read().unwrap();
            for (i, (uri, content)) in items.iter().enumerate() {
                if let Some(&id) = uri_map.get(uri) {
                    result_ids[i] = id;
                } else {
                    new_items.push(content.clone());
                    new_indices.push(i);
                }
            }
        }

        if new_items.is_empty() {
            return Ok(result_ids);
        }

        // Generate embeddings via HuggingFace API
        let embeddings = self.embed_batch(new_items).await?;

        let mut ids_to_add = Vec::new();
        let mut searcher = hnsw::Searcher::default();

        // Add to HNSW index and maps
        {
            let mut index = self.index.write().unwrap();
            let mut uri_map = self.uri_to_id.write().unwrap();
            let mut id_map = self.id_to_uri.write().unwrap();
            let mut embs = self.embeddings.write().unwrap();

            for (i, embedding) in embeddings.into_iter().enumerate() {
                let original_idx = new_indices[i];
                let uri = &items[original_idx].0;

                // Double check if inserted in race condition
                if let Some(&id) = uri_map.get(uri) {
                    result_ids[original_idx] = id;
                    continue;
                }

                let id = index.insert(embedding, &mut searcher);
                uri_map.insert(uri.clone(), id);
                id_map.insert(id, uri.clone());

                embs.push(VectorEntry {
                    uri: uri.clone(),
                    embedding: embedding.to_vec(),
                });

                result_ids[original_idx] = id;
                ids_to_add.push(id);
            }
        }

        if !ids_to_add.is_empty() {
            let _ = self.save_vectors(); // Best effort persistence
        }

        Ok(result_ids)
    }

    /// Search for similar vectors
    pub async fn search(&self, query: &str, k: usize) -> Result<Vec<SearchResult>> {
        // Generate query embedding via HuggingFace API
        let query_embedding = self.embed(query).await?;

        // Search HNSW index
        let mut searcher = hnsw::Searcher::default();
        let mut neighbors = vec![
            space::Neighbor {
                index: 0,
                distance: u32::MAX
            };
            k
        ];

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
                    let score_f32 = (neighbor.distance as f32) / 1_000_000.0;
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
        let current_uris: std::collections::HashSet<_> =
            self.uri_to_id.read().unwrap().keys().cloned().collect();

        // Filter to only current URIs
        let active_entries: Vec<_> = embeddings
            .iter()
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
            if entry.embedding.len() == 384 {
                let mut emb = [0.0f32; 384];
                emb.copy_from_slice(&entry.embedding);
                let id = new_index.insert(emb, &mut searcher);
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
