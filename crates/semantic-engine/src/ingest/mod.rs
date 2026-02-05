use crate::store::SynapseStore;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::sync::Arc;

pub mod extractor;
pub mod processor;

use extractor::{CsvExtractor, Extractor, MarkdownExtractor};

pub struct IngestionEngine {
    store: Arc<SynapseStore>,
}

impl IngestionEngine {
    pub fn new(store: Arc<SynapseStore>) -> Self {
        Self { store }
    }

    pub async fn ingest_file(&self, path: &Path, _namespace: &str) -> Result<u32> {
        let content =
            fs::read_to_string(path).with_context(|| format!("Failed to read file: {:?}", path))?;

        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

        let result = match extension {
            "csv" => CsvExtractor::new().extract(&content)?,
            "md" | "markdown" => MarkdownExtractor.extract(&content)?,
            _ => anyhow::bail!("Unsupported file type: {}", extension),
        };

        let (added, _) = self.store.ingest_triples(result.triples).await?;
        Ok(added)
    }
}
