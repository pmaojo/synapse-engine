use anyhow::{Result, Context};
use std::path::Path;
use std::fs;
use crate::store::SynapseStore;
use std::sync::Arc;

pub mod processor;
pub mod extractor;

use extractor::{Extractor, CsvExtractor, MarkdownExtractor};

pub struct IngestionEngine {
    store: Arc<SynapseStore>,
}

impl IngestionEngine {
    pub fn new(store: Arc<SynapseStore>) -> Self {
        Self { store }
    }

    pub async fn ingest_file(&self, path: &Path, namespace: &str) -> Result<u32> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {:?}", path))?;
        
        let extension = path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        let result = match extension {
            "csv" => CsvExtractor::new().extract(&content)?,
            "md" | "markdown" => MarkdownExtractor.extract(&content)?,
            _ => anyhow::bail!("Unsupported file type: {}", extension),
        };

        let (added, _) = self.store.ingest_triples(result.triples).await?;
        Ok(added)
    }
}
