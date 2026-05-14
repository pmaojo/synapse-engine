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

        let res = match extension.as_str() {
            "md" | "markdown" => self.ingest_markdown(path, namespace).await,
            "csv" => self.ingest_csv(path, namespace).await,
            "owl" | "ttl" | "rdf" | "xml" => {
                let count = ontology::OntologyLoader::load_file(&self.store, path).await?;
                Ok(count as u32)
            }
            _ => Err(anyhow::anyhow!("Unsupported file type: {}", extension)),
        };

        if res.is_ok() {
            if let Err(e) = self.generate_cache_digest() {
                eprintln!("Warning: Failed to generate cache digest: {}", e);
            }
        }

        res
    }

    async fn ingest_markdown(&self, path: &Path, role: &str) -> Result<u32> {
        use crate::md_sync::parser::MarkdownDocument;
        use oxigraph::model::{NamedNode, Quad, Subject, Term, GraphName};
        let content = std::fs::read_to_string(path)?;

        let doc = MarkdownDocument::parse(path, &content)?;
        let quads = doc.to_quads();
        let mut added = 0;

        let p_role = NamedNode::new("http://www.w3.org/ns/prov#Role").unwrap();

        for quad in quads {
            let modified_quad = quad.clone();
            // If the ingest request specified an elevated role (e.g. CoreSpecification), we inject it
            if role != "default" {
                if let GraphName::NamedNode(batch_node) = &quad.graph_name {
                    let o_role = Term::Literal(oxigraph::model::Literal::new_simple_literal(role));
                    let role_quad = Quad::new(
                        Subject::NamedNode(batch_node.clone()),
                        p_role.clone(),
                        o_role,
                        GraphName::DefaultGraph
                    );
                    let _ = self.store.store.insert(&role_quad);
                }
            }

            if self.store.store.insert(&modified_quad)? {
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

    fn generate_cache_digest(&self) -> Result<()> {
        let query = "
            PREFIX prov: <http://www.w3.org/ns/prov#>
            PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

            SELECT ?s ?time (COUNT(?p) as ?density)
            WHERE {
                GRAPH ?g {
                    ?s ?p ?o .
                    ?g prov:generatedAtTime ?time .
                }
            }
            GROUP BY ?s ?time
            ORDER BY DESC(?time) DESC(?density)
            LIMIT 5
        ";

        let result = self.store.query_sparql(query)?;
        let mut content = String::from("# Synapse Semantic Cache Digest\n\n_Auto-generated \"Morning Briefing\" of recent hot paths._\n\n## Most Recently Modified Entities:\n");

        if let Ok(json_res) = serde_json::from_str::<serde_json::Value>(&result) {
            if let serde_json::Value::Array(arr) = json_res {
                if arr.is_empty() {
                    content.push_str("No recent activity found.\n");
                } else {
                    for row in arr {
                        let s = row.get("s").and_then(|v| v.as_str()).unwrap_or("Unknown").trim_matches('"');
                        let time = row.get("time").and_then(|v| v.as_str()).unwrap_or("Unknown").trim_matches('"');
                        let density = row.get("density").and_then(|v| v.as_str()).unwrap_or("0").trim_matches('"');

                        // Try to get type or name if possible, simplified for the digest
                        content.push_str(&format!("* **{}** (Edges: {}, Last Modified: {})\n", s, density, time));
                    }
                }
            }
        } else {
            content.push_str("Error parsing SPARQL results.\n");
        }

        let synapse_dir = Path::new(".synapse/state");
        std::fs::create_dir_all(synapse_dir)?;
        let digest_path = synapse_dir.join("current_digest.md");
        std::fs::write(&digest_path, content)?;

        Ok(())
    }
}
