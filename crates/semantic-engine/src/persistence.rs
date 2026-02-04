use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use crate::properties::PropertyStore;

#[derive(Serialize, Deserialize)]
pub struct GraphSnapshot {
    pub nodes: Vec<(u32, String)>,  // (id, name)
    pub edges: Vec<(u32, u32, u16, u32)>, // (from, to, predicate_id, edge_id)
    pub predicates: Vec<(u16, String)>, // (id, name)
    pub edge_properties: PropertyStore,
    pub next_edge_id: u32,
}

impl GraphSnapshot {
    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        let data = bincode::serialize(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(path, data)?;
        println!("💾 Graph saved to {}", path);
        Ok(())
    }
    
    pub fn load_from_file(path: &str) -> std::io::Result<Self> {
        if !Path::new(path).exists() {
            return Ok(GraphSnapshot {
                nodes: Vec::new(),
                edges: Vec::new(),
                predicates: Vec::new(),
                edge_properties: PropertyStore::new(),
                next_edge_id: 0,
            });
        }
        
        let data = fs::read(path)?;
        // Try deserializing, if it fails, it might be the old format.
        // For this task, we assume we can break format or just return error.
        let snapshot: GraphSnapshot = bincode::deserialize(&data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        println!("📂 Graph loaded from {}", path);
        Ok(snapshot)
    }
}
