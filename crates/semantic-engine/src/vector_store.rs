use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use hnsw::Hnsw;
use rand_pcg::Pcg64;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};

const DEFAULT_DIMENSIONS: usize = 384;
const DEFAULT_AUTO_SAVE_THRESHOLD: usize = 100;

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

/// Vector store using Local FastEmbed for embeddings
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
    /// Local embedding model
    model: TextEmbedding,
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

        // Initialize FastEmbed model
        let mut model_opts =
            InitOptions::new(EmbeddingModel::BGESmallENV15).with_show_download_progress(true);

        if let Ok(cache_path) = std::env::var("FASTEMBED_CACHE_PATH") {
            model_opts = model_opts.with_cache_dir(PathBuf::from(cache_path));
        }

        let model = TextEmbedding::try_new(model_opts)?;

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
            model,
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
        Ok(embeddings[0].clone())
    }

    pub async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let embeddings = self.model.embed(texts, None)?;
        Ok(embeddings)
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
        let mut ids_to_add = Vec::new();
        let mut searcher = hnsw::Searcher::default();

        {
            let mut index = self.index.write().unwrap();
            let mut key_map = self.key_to_id.write().unwrap();
            let mut id_map = self.id_to_key.write().unwrap();
            let mut metadata_map = self.key_to_metadata.write().unwrap();
            let mut embs = self.embeddings.write().unwrap();

            for (i, embedding) in embeddings.into_iter().enumerate() {
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
