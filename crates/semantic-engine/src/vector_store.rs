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
const DEFAULT_DIMENSIONS: usize = 384;

/// Euclidean distance metric for HNSW
#[derive(Default, Clone)]
pub struct Euclidian;

impl space::Metric<Vec<f32>> for Euclidian {
    type Unit = u32;
    fn distance(&self, a: &Vec<f32>, b: &Vec<f32>) -> u32 {
        let len = a.len().min(b.len());
        let mut dist_sq = 0.0;
        for i in 0..len {
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
    /// Unique identifier for this vector (could be URI or Hash)
    key: String,
    embedding: Vec<f32>,
    /// Optional metadata associated with the vector
    #[serde(default)]
    metadata: serde_json::Value,
}

/// Vector store using HuggingFace Inference API for embeddings
pub struct VectorStore {
    /// HNSW index for fast approximate nearest neighbor search
    index: Arc<RwLock<Hnsw<Euclidian, Vec<f32>, Pcg64, 16, 32>>>,
    /// Mapping from node ID (internal) to Key
    id_to_key: Arc<RwLock<HashMap<usize, String>>>,
    /// Mapping from Key to node ID (internal)
    key_to_id: Arc<RwLock<HashMap<String, usize>>>,
    /// Mapping from Key to Metadata (for fast retrieval)
    key_to_metadata: Arc<RwLock<HashMap<String, serde_json::Value>>>,
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
    /// The unique key
    pub key: String,
    pub score: f32,
    /// Metadata (including original URI if applicable)
    pub metadata: serde_json::Value,
    // Helper to access URI from metadata if it exists, for backward compatibility
    pub uri: String,
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
        let mut id_to_key = HashMap::new();
        let mut key_to_id = HashMap::new();
        let mut key_to_metadata = HashMap::new();
        let mut embeddings = Vec::new();

        // Try to load persisted vectors
        if let Some(ref path) = storage_path {
            let vectors_path = path.join("vectors.json");
            if vectors_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&vectors_path) {
                    // Try new format first
                    if let Ok(data) = serde_json::from_str::<VectorData>(&content) {
                        let mut searcher = hnsw::Searcher::default();
                        for entry in data.entries {
                            if entry.embedding.len() == dimensions {
                                let id = index.insert(entry.embedding.clone(), &mut searcher);
                                id_to_key.insert(id, entry.key.clone());
                                key_to_id.insert(entry.key.clone(), id);
                                key_to_metadata.insert(entry.key.clone(), entry.metadata.clone());
                                embeddings.push(entry);
                            }
                        }
                        eprintln!("Loaded {} vectors from disk (dim={})", embeddings.len(), dimensions);
                    } else {
                        // Fallback: Try loading old format (VectorEntry with 'uri' instead of 'key')
                        #[derive(Serialize, Deserialize)]
                        struct OldVectorData { entries: Vec<OldVectorEntry> }
                        #[derive(Serialize, Deserialize)]
                        struct OldVectorEntry { uri: String, embedding: Vec<f32> }

                        if let Ok(old_data) = serde_json::from_str::<OldVectorData>(&content) {
                             let mut searcher = hnsw::Searcher::default();
                             for old in old_data.entries {
                                 if old.embedding.len() == dimensions {
                                     let id = index.insert(old.embedding.clone(), &mut searcher);
                                     id_to_key.insert(id, old.uri.clone());
                                     key_to_id.insert(old.uri.clone(), id);
                                     let metadata = serde_json::json!({ "uri": old.uri });
                                     key_to_metadata.insert(old.uri.clone(), metadata.clone());
                                     embeddings.push(VectorEntry {
                                         key: old.uri.clone(),
                                         embedding: old.embedding,
                                         metadata,
                                     });
                                 }
                             }
                             eprintln!("Loaded {} legacy vectors from disk (dim={})", embeddings.len(), dimensions);
                        }
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
            id_to_key: Arc::new(RwLock::new(id_to_key)),
            key_to_id: Arc::new(RwLock::new(key_to_id)),
            key_to_metadata: Arc::new(RwLock::new(key_to_metadata)),
            storage_path,
            client,
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
    /// (Mocked if MOCK_EMBEDDINGS is set)
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        if std::env::var("MOCK_EMBEDDINGS").is_ok() {
            // Return random embedding for testing
            use rand::Rng;
            let mut rng = rand::rng();
            let vec: Vec<f32> = (0..self.dimensions).map(|_| rng.random()).collect();
            return Ok(vec);
        }

        let embeddings = self.embed_batch(vec![text.to_string()]).await?;
        Ok(embeddings[0].clone())
    }

    /// Generate embeddings for multiple texts using HuggingFace Inference API
    pub async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        if std::env::var("MOCK_EMBEDDINGS").is_ok() {
             use rand::Rng;
             let mut rng = rand::rng();
             let mut results = Vec::new();
             for _ in 0..texts.len() {
                 let vec: Vec<f32> = (0..self.dimensions).map(|_| rng.random()).collect();
                 results.push(vec);
             }
             return Ok(results);
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

                if vec.len() != self.dimensions {
                    anyhow::bail!("Expected {} dimensions, got {}", self.dimensions, vec.len());
                }
                results.push(vec);
            }
        } else {
            // Handle case where we sent 1 text and got single flat array [0.1, ...]
            if texts.len() == 1 {
                if let Ok(vec) = serde_json::from_value::<Vec<f32>>(response_json) {
                    if vec.len() == self.dimensions {
                        results.push(vec);
                    }
                }
            }
        }

        if results.len() != texts.len() {
            anyhow::bail!("Expected {} embeddings, got {}", texts.len(), results.len());
        }

        Ok(results)
    }

    /// Add a key with its text content to the index
    pub async fn add(&self, key: &str, content: &str, metadata: serde_json::Value) -> Result<usize> {
        let results = self.add_batch(vec![(key.to_string(), content.to_string(), metadata)]).await?;
        Ok(results[0])
    }

    /// Add multiple keys with their text content to the index
    pub async fn add_batch(&self, items: Vec<(String, String, serde_json::Value)>) -> Result<Vec<usize>> {
        // Filter out existing keys
        let mut new_items = Vec::new();
        let mut result_ids = vec![0; items.len()];
        let mut new_indices = Vec::new(); // Map index in new_items to index in items

        {
            let key_map = self.key_to_id.read().unwrap();
            for (i, (key, content, _)) in items.iter().enumerate() {
                if let Some(&id) = key_map.get(key) {
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
            let mut key_map = self.key_to_id.write().unwrap();
            let mut id_map = self.id_to_key.write().unwrap();
            let mut metadata_map = self.key_to_metadata.write().unwrap();
            let mut embs = self.embeddings.write().unwrap();

            for (i, embedding) in embeddings.into_iter().enumerate() {
                let original_idx = new_indices[i];
                let (key, _, metadata) = &items[original_idx];

                // Double check if inserted in race condition
                if let Some(&id) = key_map.get(key) {
                    result_ids[original_idx] = id;
                    continue;
                }

                let id = index.insert(embedding.clone(), &mut searcher);
                key_map.insert(key.clone(), id);
                id_map.insert(id, key.clone());
                metadata_map.insert(key.clone(), metadata.clone());

                embs.push(VectorEntry {
                    key: key.clone(),
                    embedding: embedding,
                    metadata: metadata.clone(),
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
        let id_map = self.id_to_key.read().unwrap();
        let metadata_map = self.key_to_metadata.read().unwrap();

        let results: Vec<SearchResult> = found_neighbors
            .iter()
            .filter_map(|neighbor| {
                id_map.get(&neighbor.index).map(|key| {
                    let score_f32 = (neighbor.distance as f32) / 1_000_000.0;

                    let metadata = metadata_map.get(key).cloned().unwrap_or(serde_json::Value::Null);

                    let uri = metadata.get("uri").and_then(|v| v.as_str()).unwrap_or(key).to_string();

                    SearchResult {
                        key: key.clone(),
                        score: 1.0 / (1.0 + score_f32),
                        metadata,
                        uri,
                    }
                })
            })
            .collect();

        Ok(results)
    }

    pub fn get_key(&self, id: usize) -> Option<String> {
        self.id_to_key.read().unwrap().get(&id).cloned()
    }

    pub fn get_id(&self, key: &str) -> Option<usize> {
        self.key_to_id.read().unwrap().get(key).copied()
    }

    pub fn len(&self) -> usize {
        self.key_to_id.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Compaction: rebuild index from stored embeddings, removing stale entries
    pub fn compact(&self) -> Result<usize> {
        let embeddings = self.embeddings.read().unwrap();
        let current_keys: std::collections::HashSet<_> = self.key_to_id.read().unwrap().keys().cloned().collect();
        
        if current_keys.is_empty() && !embeddings.is_empty() {
            return Ok(0);
        }

        // Filter to only current Keys
        let active_entries: Vec<_> = embeddings
            .iter()
            .filter(|e| current_keys.contains(&e.key))
            .cloned()
            .collect();

        let removed = embeddings.len() - active_entries.len();

        if removed == 0 {
            return Ok(0);
        }

        // Rebuild index
        let mut new_index = hnsw::Hnsw::new(Euclidian);
        let mut new_id_to_key = std::collections::HashMap::new();
        let mut new_key_to_id = std::collections::HashMap::new();
        let mut new_key_to_metadata = std::collections::HashMap::new();
        let mut searcher = hnsw::Searcher::default();

        for entry in &active_entries {
            if entry.embedding.len() == self.dimensions {
                let id = new_index.insert(entry.embedding.clone(), &mut searcher);
                new_id_to_key.insert(id, entry.key.clone());
                new_key_to_id.insert(entry.key.clone(), id);
                new_key_to_metadata.insert(entry.key.clone(), entry.metadata.clone());
            }
        }

        // Swap in new index
        *self.index.write().unwrap() = new_index;
        *self.id_to_key.write().unwrap() = new_id_to_key;
        *self.key_to_id.write().unwrap() = new_key_to_id;
        *self.key_to_metadata.write().unwrap() = new_key_to_metadata;

        // Update embeddings (drop takes write lock)
        drop(embeddings);
        *self.embeddings.write().unwrap() = active_entries;

        let _ = self.save_vectors();

        Ok(removed)
    }

    /// Remove a Key from the vector store
    pub fn remove(&self, key: &str) -> bool {
        let mut key_map = self.key_to_id.write().unwrap();
        let mut id_map = self.id_to_key.write().unwrap();
        let mut metadata_map = self.key_to_metadata.write().unwrap();

        if let Some(id) = key_map.remove(key) {
            id_map.remove(&id);
            metadata_map.remove(key);
            // Note: actual index entry remains until compaction
            true
        } else {
            false
        }
    }

    /// Get storage stats
    pub fn stats(&self) -> (usize, usize, usize) {
        let embeddings_count = self.embeddings.read().unwrap().len();
        let active_count = self.key_to_id.read().unwrap().len();
        let stale_count = embeddings_count.saturating_sub(active_count);
        (active_count, stale_count, embeddings_count)
    }
}
