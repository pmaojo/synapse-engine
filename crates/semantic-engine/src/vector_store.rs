use anyhow::Result;
use hnsw::Hnsw;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use rand_pcg::Pcg64;

const HUGGINGFACE_API_URL: &str = "https://api-inference.huggingface.co/pipeline/feature-extraction";
const DEFAULT_MODEL: &str = "sentence-transformers/all-MiniLM-L6-v2"; // 384 dims, fast

/// Euclidean distance metric for HNSW
#[derive(Default, Clone)]
pub struct Euclidian;

impl space::Metric<[f32; 384]> for Euclidian {
    type Unit = u64;
    fn distance(&self, a: &[f32; 384], b: &[f32; 384]) -> u64 {
        let mut dist_sq = 0.0;
        for i in 0..384 {
            let diff = a[i] - b[i];
            dist_sq += diff * diff;
        }
        // Floating point to bits for ordered comparison as per space v0.17 recommendations
        dist_sq.sqrt().to_bits() as u64
    }
}

/// Vector store using HuggingFace Inference API for embeddings
pub struct VectorStore {
    /// HNSW index for fast approximate nearest neighbor search
    index: Arc<RwLock<Hnsw<Euclidian, [f32; 384], Pcg64, 16, 32>>>,
    /// Mapping from node ID to URI
    id_to_uri: Arc<RwLock<HashMap<usize, String>>>,
    /// Mapping from URI to node ID
    uri_to_id: Arc<RwLock<HashMap<String, usize>>>,
    /// HTTP client for HuggingFace API
    client: Client,
    /// HuggingFace API token (optional, for rate limits)
    api_token: Option<String>,
    /// Model name
    model: String,
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
    pub fn new(_namespace: &str) -> Result<Self> {
        // Create HNSW index with Euclidian metric
        let index = Hnsw::new(Euclidian);

        // Get API token from environment (optional)
        let api_token = std::env::var("HUGGINGFACE_API_TOKEN").ok();

        Ok(Self {
            index: Arc::new(RwLock::new(index)),
            id_to_uri: Arc::new(RwLock::new(HashMap::new())),
            uri_to_id: Arc::new(RwLock::new(HashMap::new())),
            client: Client::new(),
            api_token,
            model: DEFAULT_MODEL.to_string(),
        })
    }

    /// Generate embedding for a text using HuggingFace Inference API
    pub async fn embed(&self, text: &str) -> Result<[f32; 384]> {
        let url = format!("{}/{}", HUGGINGFACE_API_URL, self.model);
        
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
        
        if embedding_vec.len() != 384 {
            anyhow::bail!("Expected 384 dimensions, got {}", embedding_vec.len());
        }

        let mut embedding = [0.0f32; 384];
        embedding.copy_from_slice(&embedding_vec[0..384]);
        
        Ok(embedding)
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
            index.insert(embedding, &mut searcher)
        };

        // Update mappings
        {
            let mut uri_map = self.uri_to_id.write().unwrap();
            let mut id_map = self.id_to_uri.write().unwrap();
            uri_map.insert(uri.to_string(), id);
            id_map.insert(id, uri.to_string());
        }

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
}
