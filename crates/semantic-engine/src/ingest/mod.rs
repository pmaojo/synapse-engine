pub mod extractor;
pub mod ontology;

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
            "md" | "markdown" => self.ingest_markdown(path).await,
            "csv" => self.ingest_csv(path, namespace).await,
            "owl" | "ttl" | "rdf" | "xml" => {
                let count = ontology::OntologyLoader::load_file(&self.store, path).await?;
                Ok(count as u32)
            }
            _ => Err(anyhow::anyhow!("Unsupported file type: {}", extension)),
        }
    }

    async fn ingest_markdown(&self, path: &Path) -> Result<u32> {
        use crate::md_sync::parser::MarkdownDocument;
        let content = std::fs::read_to_string(path)?;

        let doc = MarkdownDocument::parse(path, &content)?;
        let quads = doc.to_quads();
        let mut added = 0;

        for quad in quads {
            if self.store.store.insert(&quad)? {
                added += 1;
            }
        }

        // El motor lógic se actualizará durante la fase 4 (SynapseReasoner)
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
