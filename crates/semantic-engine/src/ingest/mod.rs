pub mod extractor;
pub mod ontology;
pub mod processor;
use crate::store::{IngestTriple, SynapseStore};
use anyhow::Result;
use std::path::Path;

pub struct IngestionEngine {
    store: std::sync::Arc<SynapseStore>,
}

impl IngestionEngine {
    pub fn new(store: std::sync::Arc<SynapseStore>) -> Self {
        Self { store }
    }

    pub async fn ingest_file(&self, path: &Path, namespace: &str) -> Result<u32> {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "md" | "markdown" => self.ingest_markdown(path, namespace).await,
            "csv" => self.ingest_csv(path, namespace).await,
            "owl" | "ttl" | "rdf" | "xml" => {
                let count = ontology::OntologyLoader::load_file(&self.store, path).await?;
                Ok(count as u32)
            }
            _ => Err(anyhow::anyhow!("Unsupported file type: {}", extension)),
        }
    }

    async fn ingest_markdown(&self, path: &Path, namespace: &str) -> Result<u32> {
        let content = std::fs::read_to_string(path)?;
        let triples = extractor::extract_metadata(&content, path.to_str().unwrap());

        let ingest_triples: Vec<IngestTriple> = triples
            .into_iter()
            .map(|t| IngestTriple {
                subject: t.subject,
                predicate: t.predicate,
                object: t.object,
                provenance: Some(crate::store::Provenance {
                    source: path.to_string_lossy().to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    method: "markdown_extractor".to_string(),
                }),
            })
            .collect();

        let (added, _) = self.store.ingest_triples(ingest_triples).await?;

        // Also ingest content into vector store for RAG
        if let Some(ref vs) = self.store.vector_store {
            let processor = super::processor::TextProcessor::new();
            let chunks = processor.chunk_text(&content, 1000, 150);
            for (i, chunk) in chunks.iter().enumerate() {
                let chunk_uri = format!("{}#chunk-{}", path.to_string_lossy(), i);
                let metadata = serde_json::json!({
                    "uri": path.to_string_lossy(),
                    "chunk_uri": chunk_uri,
                    "type": "markdown_chunk",
                    "namespace": namespace
                });
                if let Err(e) = vs.add(&chunk_uri, chunk, metadata).await {
                    eprintln!("Failed to index chunk {}: {}", i, e);
                }
            }
        }

        Ok(added)
    }

    async fn ingest_csv(&self, path: &Path, _namespace: &str) -> Result<u32> {
        let mut reader = csv::Reader::from_path(path)?;
        let headers = reader.headers()?.clone();

        let mut triples = Vec::new();
        let filename = path.file_name().unwrap().to_string_lossy();

        for result in reader.records() {
            let record = result?;
            // Assume first column is ID/Subject
            if let Some(subject) = record.get(0) {
                let subject_uri = format!("urn:csv:{}:{}", filename, subject); // basic namespacing

                for (j, field) in record.iter().enumerate().skip(1) {
                    if let Some(header) = headers.get(j) {
                        if !field.is_empty() {
                            triples.push(IngestTriple {
                                subject: subject_uri.clone(),
                                predicate: format!("urn:csv:prop:{}", header),
                                object: field.to_string(),
                                provenance: Some(crate::store::Provenance {
                                    source: path.to_string_lossy().to_string(),
                                    timestamp: chrono::Utc::now().to_rfc3339(),
                                    method: "csv_extractor".to_string(),
                                }),
                            });
                        }
                    }
                }
            }
        }

        let (added, _) = self.store.ingest_triples(triples).await?;
        Ok(added)
    }
}
