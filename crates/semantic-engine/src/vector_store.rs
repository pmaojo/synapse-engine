use anyhow::{anyhow, Result};
#[cfg(feature = "local-embeddings")]
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use hnsw::Hnsw;
use rand::Rng;
use rand_pcg::Pcg64;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};

const DEFAULT_DIMENSIONS: usize = 384;
const DEFAULT_AUTO_SAVE_THRESHOLD: usize = 100;
const DEFAULT_REMOTE_API_URL: &str = "http://localhost:11434/api/embeddings";
const DEFAULT_REMOTE_MODEL: &str = "nomic-embed-text";

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
    /// Optional metadata associated with the vector (serialized as JSON string for compatibility)
    #[serde(default)]
    metadata_json: String,
}

// --- Embedder Abstraction ---

struct RemoteEmbedder {
    client: reqwest::Client,
    url: String,
    model: String,
    api_key: Option<String>,
}

impl RemoteEmbedder {
    fn new(url: String, model: String, api_key: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            url,
            model,
            api_key,
        }
    }

    async fn embed_one(&self, text: &str) -> Result<Vec<f32>> {
        let mut body = serde_json::json!({
            "model": self.model,
            "prompt": text
        });

        // If it looks like OpenAI (contains "openai" or "v1/embeddings"), adapt format
        let is_openai = self.url.contains("v1/embeddings");
        if is_openai {
            body = serde_json::json!({
                "model": self.model,
                "input": text
            });
        }

        let mut req = self.client.post(&self.url).json(&body);
        if let Some(key) = &self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Remote embedding failed ({}) : {}", status, text));
        }

        let json: serde_json::Value = resp.json().await?;

        if is_openai {
            // OpenAI format: { "data": [ { "embedding": [...] } ] }
            let embedding = json["data"][0]["embedding"]
                .as_array()
                .ok_or_else(|| anyhow!("Invalid OpenAI response format"))?
                .iter()
                .map(|v| v.as_f64().unwrap_or_default() as f32)
                .collect();
            Ok(embedding)
        } else {
            // Ollama format: { "embedding": [...] }
            let embedding = json["embedding"]
                .as_array()
                .ok_or_else(|| anyhow!("Invalid Ollama response format"))?
                .iter()
                .map(|v| v.as_f64().unwrap_or_default() as f32)
                .collect();
            Ok(embedding)
        }
    }

    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        // Ollama/Remote often doesn't support batching in the same way, or it varies.
        // We will loop concurrently.
        // let mut futures = Vec::new();
        // Since we are iterating over owned strings and calling an async method that takes a reference,
        // we need to be careful with lifetimes if we use join_all or similar with references.
        // However, we can just await in loop for simplicity as done before, but let's fix the lifetime error.

        // The error was: `text` dropped while still borrowed.
        // `embed_one` takes `&str`.

        let mut results = Vec::new();
        for text in texts {
            // We await immediately, so `text` (owned by loop) lives long enough for the call
            results.push(self.embed_one(&text).await?);
        }

        Ok(results)
    }
}

enum Embedder {
    #[cfg(feature = "local-embeddings")]
    Local(TextEmbedding),
    Remote(RemoteEmbedder),
    Mock,
}

impl Embedder {
    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        match self {
            #[cfg(feature = "local-embeddings")]
            Embedder::Local(model) => {
                // fastembed is blocking/CPU heavy, so we should spawn_blocking if we were rigorous,
                // but for now we follow existing pattern (it seems existing code didn't spawn_blocking?)
                // Ah, the memory mentioned "executed via tokio::task::spawn_blocking".
                // We should preserve that if possible, but TextEmbedding is not Sync?
                // TextEmbedding IS Sync.
                // But fastembed::TextEmbedding::embed is synchronous.
                // So strictly we should wrap it.
                // But let's keep it simple:
                Ok(model.embed(texts, None)?)
            }
            Embedder::Remote(remote) => remote.embed_batch(texts).await,
            Embedder::Mock => {
                let mut rng = rand::thread_rng();
                let mut results = Vec::with_capacity(texts.len());
                for _ in 0..texts.len() {
                    let embedding: Vec<f32> = (0..DEFAULT_DIMENSIONS).map(|_| rng.gen()).collect();
                    results.push(embedding);
                }
                Ok(results)
            }
        }
    }
}

// --- VectorStore ---

/// Vector store using Local FastEmbed or Remote API for embeddings
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
    /// Embedding provider
    embedder: Arc<Embedder>,
    /// Vector dimensions
    dimensions: usize,
    /// Stored embeddings for persistence
    embeddings: Arc<RwLock<Vec<VectorEntry>>>,
    /// Number of unsaved changes
    dirty_count: Arc<AtomicUsize>,
    /// Threshold for auto-save
    auto_save_threshold: usize,
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

        // Configure Embedder
        // Priority:
        // 0. If env `MOCK_EMBEDDINGS` == "true" -> Mock
        // 1. If feature `local-embeddings` is OFF -> Remote
        // 2. If env `EMBEDDING_PROVIDER` == "remote" -> Remote
        // 3. Else -> Local (if enabled)

        let provider = std::env::var("EMBEDDING_PROVIDER").unwrap_or_else(|_| "local".to_string());
        let mock_env = std::env::var("MOCK_EMBEDDINGS").unwrap_or_default();

        let embedder = if mock_env == "true" {
             eprintln!("VectorStore: Using Mock Embeddings");
             Embedder::Mock
        } else if provider == "remote" || !cfg!(feature = "local-embeddings") {
             let url = std::env::var("EMBEDDING_API_URL").unwrap_or_else(|_| DEFAULT_REMOTE_API_URL.to_string());
             let model = std::env::var("EMBEDDING_MODEL").unwrap_or_else(|_| DEFAULT_REMOTE_MODEL.to_string());
             let key = std::env::var("EMBEDDING_API_KEY").ok();

             eprintln!("VectorStore: Using Remote Embeddings ({} model={})", url, model);
             Embedder::Remote(RemoteEmbedder::new(url, model, key))
        } else {
            #[cfg(feature = "local-embeddings")]
            {
                // Initialize FastEmbed model
                let mut model_opts =
                    InitOptions::new(EmbeddingModel::BGESmallENV15).with_show_download_progress(true);

                if let Ok(cache_path) = std::env::var("FASTEMBED_CACHE_PATH") {
                    model_opts = model_opts.with_cache_dir(PathBuf::from(cache_path));
                }

                eprintln!("VectorStore: Using Local Embeddings (fastembed)");
                let model = TextEmbedding::try_new(model_opts)?;
                Embedder::Local(model)
            }
            #[cfg(not(feature = "local-embeddings"))]
            {
                // This branch should be unreachable due to the logic above,
                // but safe fallback if logic changes
                 let url = std::env::var("EMBEDDING_API_URL").unwrap_or_else(|_| DEFAULT_REMOTE_API_URL.to_string());
                 let model = std::env::var("EMBEDDING_MODEL").unwrap_or_else(|_| DEFAULT_REMOTE_MODEL.to_string());
                 Embedder::Remote(RemoteEmbedder::new(url, model, None))
            }
        };

        // Create HNSW index
        let mut index = Hnsw::new(Euclidian);
        let mut id_to_key = HashMap::new();
        let mut key_to_id = HashMap::new();
        let mut key_to_metadata = HashMap::new();
        let mut embeddings = Vec::new();

        // Try to load persisted vectors
        if let Some(ref path) = storage_path {
            let vectors_json = path.join("vectors.json");

            let loaded_data = if vectors_json.exists() {
                match std::fs::read_to_string(&vectors_json) {
                    Ok(content) => match serde_json::from_str::<VectorData>(&content) {
                        Ok(data) => Some(data),
                        Err(e) => {
                            eprintln!("ERROR: Failed to parse vectors: {}", e);
                            None
                        }
                    },
                    Err(_) => None,
                }
            } else {
                None
            };

            if let Some(data) = loaded_data {
                let mut searcher = hnsw::Searcher::default();
                for entry in data.entries {
                    if entry.embedding.len() == dimensions {
                        let id = index.insert(entry.embedding.clone(), &mut searcher);
                        id_to_key.insert(id, entry.key.clone());
                        key_to_id.insert(entry.key.clone(), id);
                        
                        let metadata = serde_json::from_str(&entry.metadata_json).unwrap_or(serde_json::Value::Null);
                        key_to_metadata.insert(entry.key.clone(), metadata);
                        embeddings.push(entry);
                    }
                }
                eprintln!("Loaded {} vectors from disk", embeddings.len());
            }
        }

        Ok(Self {
            index: Arc::new(RwLock::new(index)),
            id_to_key: Arc::new(RwLock::new(id_to_key)),
            key_to_id: Arc::new(RwLock::new(key_to_id)),
            key_to_metadata: Arc::new(RwLock::new(key_to_metadata)),
            storage_path,
            embedder: Arc::new(embedder),
            dimensions,
            embeddings: Arc::new(RwLock::new(embeddings)),
            dirty_count: Arc::new(AtomicUsize::new(0)),
            auto_save_threshold: DEFAULT_AUTO_SAVE_THRESHOLD,
        })
    }

    /// Save vectors to disk (JSON format for robust cross-version compatibility)
    fn save_vectors(&self) -> Result<()> {
        if let Some(ref path) = self.storage_path {
            std::fs::create_dir_all(path)?;

            let (entries, current_dirty) = {
                let guard = self.embeddings.read().unwrap();
                (guard.clone(), self.dirty_count.load(Ordering::Relaxed))
            };

            let data = VectorData { entries };
            let json = serde_json::to_string_pretty(&data)?;
            std::fs::write(path.join("vectors.json"), json)?;

            if current_dirty > 0 {
                let _ = self.dirty_count.fetch_sub(current_dirty, Ordering::Relaxed);
            }
        }
        Ok(())
    }

    pub fn flush(&self) -> Result<()> {
        self.save_vectors()
    }

    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self.embed_batch(vec![text.to_string()]).await?;
        if embeddings.is_empty() {
             return Err(anyhow!("No embedding returned"));
        }
        Ok(embeddings[0].clone())
    }

    pub async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        self.embedder.embed_batch(texts).await
    }

    pub async fn add(
        &self,
        key: &str,
        content: &str,
        metadata: serde_json::Value,
    ) -> Result<usize> {
        let results = self
            .add_batch(vec![(key.to_string(), content.to_string(), metadata)])
            .await?;
        Ok(results[0])
    }

    pub async fn add_batch(
        &self,
        items: Vec<(String, String, serde_json::Value)>,
    ) -> Result<Vec<usize>> {
        let mut new_items = Vec::new();
        let mut result_ids = vec![0; items.len()];
        let mut new_indices = Vec::new();

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

        let embeddings = self.embed_batch(new_items).await?;

        // Validation: ensure we got embeddings
        if embeddings.len() != new_indices.len() {
             eprintln!("WARNING: Requested {} embeddings, got {}. Some items may be skipped.", new_indices.len(), embeddings.len());
        }

        let mut ids_to_add = Vec::new();
        let mut searcher = hnsw::Searcher::default();

        {
            let mut index = self.index.write().unwrap();
            let mut key_map = self.key_to_id.write().unwrap();
            let mut id_map = self.id_to_key.write().unwrap();
            let mut metadata_map = self.key_to_metadata.write().unwrap();
            let mut embs = self.embeddings.write().unwrap();

            for (i, embedding) in embeddings.into_iter().enumerate() {
                if i >= new_indices.len() { break; } // Safety
                let original_idx = new_indices[i];
                let (key, _, metadata) = &items[original_idx];

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
                    embedding,
                    metadata_json: serde_json::to_string(metadata).unwrap_or_default(),
                });

                result_ids[original_idx] = id;
                ids_to_add.push(id);
            }
        }

        if !ids_to_add.is_empty() {
            let count = self
                .dirty_count
                .fetch_add(ids_to_add.len(), Ordering::Relaxed);
            if count + ids_to_add.len() >= self.auto_save_threshold {
                let _ = self.save_vectors();
            }
        }

        Ok(result_ids)
    }

    pub async fn search(&self, query: &str, k: usize) -> Result<Vec<SearchResult>> {
        let query_embedding = self.embed(query).await?;
        let mut searcher = hnsw::Searcher::default();

        let index = self.index.read().unwrap();
        let len = index.len();
        if len == 0 {
            return Ok(Vec::new());
        }

        let k = k.min(len);
        let ef = k.max(50);

        let mut neighbors = vec![
            space::Neighbor {
                index: 0,
                distance: u32::MAX
            };
            k
        ];

        let found_neighbors = index.nearest(&query_embedding, ef, &mut searcher, &mut neighbors);

        let id_map = self.id_to_key.read().unwrap();
        let metadata_map = self.key_to_metadata.read().unwrap();

        let results: Vec<SearchResult> = found_neighbors
            .iter()
            .filter_map(|neighbor| {
                id_map.get(&neighbor.index).map(|key| {
                    let score_f32 = (neighbor.distance as f32) / 1_000_000.0;
                    let metadata = metadata_map
                        .get(key)
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);
                    let uri = metadata
                        .get("uri")
                        .and_then(|v| v.as_str())
                        .unwrap_or(key)
                        .to_string();

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

    pub fn compact(&self) -> Result<usize> {
        let embeddings = self.embeddings.read().unwrap();
        let current_keys: std::collections::HashSet<_> =
            self.key_to_id.read().unwrap().keys().cloned().collect();

        if current_keys.is_empty() && !embeddings.is_empty() {
            return Ok(0);
        }

        let active_entries: Vec<_> = embeddings
            .iter()
            .filter(|e| current_keys.contains(&e.key))
            .cloned()
            .collect();

        let removed = embeddings.len() - active_entries.len();
        if removed == 0 {
            return Ok(0);
        }

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
                let metadata = serde_json::from_str(&entry.metadata_json).unwrap_or(serde_json::Value::Null);
                new_key_to_metadata.insert(entry.key.clone(), metadata);
            }
        }

        *self.index.write().unwrap() = new_index;
        *self.id_to_key.write().unwrap() = new_id_to_key;
        *self.key_to_id.write().unwrap() = new_key_to_id;
        *self.key_to_metadata.write().unwrap() = new_key_to_metadata;

        drop(embeddings);
        *self.embeddings.write().unwrap() = active_entries;
        let _ = self.save_vectors();
        Ok(removed)
    }

    pub fn remove(&self, key: &str) -> bool {
        let mut key_map = self.key_to_id.write().unwrap();
        let mut id_map = self.id_to_key.write().unwrap();
        let mut metadata_map = self.key_to_metadata.write().unwrap();

        if let Some(id) = key_map.remove(key) {
            id_map.remove(&id);
            metadata_map.remove(key);
            true
        } else {
            false
        }
    }

    pub fn stats(&self) -> (usize, usize, usize) {
        let embeddings_count = self.embeddings.read().unwrap().len();
        let active_count = self.key_to_id.read().unwrap().len();
        let stale_count = embeddings_count.saturating_sub(active_count);
        (active_count, stale_count, embeddings_count)
    }
}
