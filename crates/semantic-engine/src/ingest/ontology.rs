use crate::store::{IngestTriple, Provenance, SynapseStore};
use anyhow::Result;
use oxigraph::io::RdfFormat;
use oxigraph::io::RdfParser;
use std::fs;
use std::path::Path;

pub struct OntologyLoader;

impl OntologyLoader {
    pub async fn load_directory(store: &SynapseStore, dir_path: &Path) -> Result<usize> {
        let mut total_triples = 0;

        if !dir_path.exists() {
            eprintln!("Ontology directory not found: {:?}", dir_path);
            return Ok(0);
        }

        let entries = fs::read_dir(dir_path)?;

        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        let ext = ext.to_lowercase();
                        if matches!(ext.as_str(), "owl" | "ttl" | "rdf" | "xml") {
                            eprintln!("Loading ontology: {:?}", path.file_name().unwrap());
                            match Self::load_file(store, &path).await {
                                Ok(count) => {
                                    total_triples += count;
                                    eprintln!("  Loaded {} triples", count);
                                }
                                Err(e) => {
                                    eprintln!("  Failed to load ontology {:?}: {}", path.display(), e);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(total_triples)
    }

    pub async fn load_file(store: &SynapseStore, path: &Path) -> Result<usize> {
        let file = fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);

        // Determine format based on extension
        let format = if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            match ext.to_lowercase().as_str() {
                "ttl" => RdfFormat::Turtle,
                "rdf" | "owl" | "xml" => RdfFormat::RdfXml,
                _ => RdfFormat::Turtle, // Default fallback
            }
        } else {
            RdfFormat::Turtle
        };

        let mut ingest_triples = Vec::new();
        let parser = RdfParser::from_format(format);

        for triple_result in parser.for_reader(reader) {
            let triple = triple_result.map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

            ingest_triples.push(IngestTriple {
                subject: triple.subject.to_string(),
                predicate: triple.predicate.to_string(),
                object: triple.object.to_string(),
                provenance: Some(Provenance {
                    source: path.file_name().unwrap().to_string_lossy().to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    method: "ontology_loader".to_string(),
                }),
            });
        }

        let count = ingest_triples.len();
        if count > 0 {
            store.ingest_triples(ingest_triples).await?;
        }

        Ok(count)
    }
}
