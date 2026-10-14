use anyhow::Result;
use oxigraph::model::*;
use oxigraph::store::Store;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Provenance {
    pub source: String,
    pub timestamp: String,
    pub method: String,
}

pub struct IngestTriple {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub provenance: Option<Provenance>,
}

pub struct SynapseStore {
    pub store: Store,
    pub namespace: String,
    pub storage_path: PathBuf,
}

impl SynapseStore {
    pub fn open(namespace: &str, storage_path: &str) -> Result<Self> {
        let path = PathBuf::from(storage_path).join(namespace);
        std::fs::create_dir_all(&path)?;

        // Oxigraph is configured with rocksdb feature in Cargo.toml
        // so Store::open utilizes rocksdb directly for persistence.
        let store = Store::open(&path)?;

        Ok(Self {
            store,
            namespace: namespace.to_string(),
            storage_path: path,
        })
    }

    pub async fn ingest_triples(&self, triples: Vec<IngestTriple>) -> Result<(u32, u32)> {
        let mut added = 0;
        let mut affected_entities = std::collections::HashSet::new();

        // Group by provenance to optimize batch insertion into named graphs
        let mut batches: HashMap<Option<Provenance>, Vec<(String, String, String)>> =
            HashMap::new();

        for t in triples {
            batches
                .entry(t.provenance)
                .or_default()
                .push((t.subject, t.predicate, t.object));
        }

        for (prov, batch_triples) in batches {
            let graph_name = if let Some(p) = &prov {
                let uuid = Uuid::new_v4();
                let uri = format!("urn:batch:{}", uuid);

                let batch_node = NamedNode::new_unchecked(&uri);
                let p_derived =
                    NamedNode::new_unchecked("http://www.w3.org/ns/prov#wasDerivedFrom");
                let p_time = NamedNode::new_unchecked("http://www.w3.org/ns/prov#generatedAtTime");
                let p_method = NamedNode::new_unchecked("http://www.w3.org/ns/prov#wasGeneratedBy");

                let o_source = Literal::new_simple_literal(&p.source);
                let o_time = Literal::new_simple_literal(&p.timestamp);
                let o_method = Literal::new_simple_literal(&p.method);

                self.store.insert(&Quad::new(
                    batch_node.clone(),
                    p_derived,
                    o_source,
                    GraphName::DefaultGraph,
                ))?;
                self.store.insert(&Quad::new(
                    batch_node.clone(),
                    p_time,
                    o_time,
                    GraphName::DefaultGraph,
                ))?;
                self.store.insert(&Quad::new(
                    batch_node.clone(),
                    p_method,
                    o_method,
                    GraphName::DefaultGraph,
                ))?;

                // If source is "mcp", put triples in default graph for easier querying
                if p.source == "mcp" {
                    GraphName::DefaultGraph
                } else {
                    GraphName::NamedNode(batch_node)
                }
            } else {
                GraphName::DefaultGraph
            };

            for (s, p, o) in batch_triples {
                let subject_uri = self.ensure_uri(&s);
                let predicate_uri = self.ensure_uri(&p);

                let (object_term, _object_key) = if o.starts_with('"') && o.ends_with('"') && o.len() >= 2 {
                    let literal_val = &o[1..o.len() - 1];
                    (Term::Literal(Literal::new_simple_literal(literal_val)), literal_val.to_string())
                } else {
                    let uri = self.ensure_uri(&o);
                    (Term::NamedNode(NamedNode::new_unchecked(&uri)), uri)
                };

                let subject = Subject::NamedNode(NamedNode::new_unchecked(&subject_uri));
                let predicate = NamedNode::new_unchecked(&predicate_uri);
                let object = object_term;

                let quad = Quad::new(subject, predicate, object, graph_name.clone());
                let inserted = self.store.insert(&quad)?;

                if inserted {
                    added += 1;
                    affected_entities.insert(subject_uri);
                }
            }
        }

        // Trigger Graph -> MD synchronization for affected entities
        self.sync_entities_to_markdown(&affected_entities);

        Ok((added, 0))
    }

    /// Fetches derived properties for given entities and syncs them back to their origin Markdown files
    pub fn sync_entities_to_markdown(&self, entities: &std::collections::HashSet<String>) {
        use crate::md_sync::writer::MarkdownWriter;

        let p_derived = NamedNode::new("http://www.w3.org/ns/prov#wasDerivedFrom").unwrap();

        for entity_uri in entities {
            // Find if this entity came from a Markdown file
            let subject = NamedNodeRef::new(entity_uri).ok();
            if let Some(s) = subject {
                let mut origin_file = None;
                for q in self.store.quads_for_pattern(Some(s.into()), Some(p_derived.as_ref()), None, None).flatten() {
                    if let Term::NamedNode(file_node) = q.object {
                        let path_str = file_node.as_str().trim_start_matches("file://");
                        origin_file = Some(path_str.to_string());
                        break;
                    }
                }

                if let Some(file_path) = origin_file {
                    if std::path::Path::new(&file_path).exists() {
                        // Gather inferences/backlinks (where this entity is the object)
                        let mut inferences = Vec::new();
                        for q in self.store.quads_for_pattern(None, None, Some(s.into()), None).flatten() {
                            if q.predicate.as_ref() != p_derived.as_ref() {
                                if let Subject::NamedNode(subj) = q.subject {
                                    inferences.push((
                                        q.predicate.as_str().to_string(),
                                        subj.as_str().to_string(),
                                    ));
                                }
                            }
                        }

                        if !inferences.is_empty() {
                            if let Err(e) = MarkdownWriter::write_inferences(&file_path, &inferences) {
                                eprintln!("Warning: Failed to sync markdown backlinks {}: {}", file_path, e);
                            }
                        }

                        // Gather direct properties to update frontmatter
                        let mut properties = std::collections::HashMap::new();
                        let p_type = NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type").unwrap();

                        for q in self.store.quads_for_pattern(Some(s.into()), None, None, None).flatten() {
                            let pred = q.predicate.as_str();
                            if pred == p_type.as_str() {
                                if let Term::NamedNode(obj) = q.object {
                                    if let Some(t) = obj.as_str().strip_prefix("urn:synapse:type:") {
                                        properties.insert("type".to_string(), t.to_string());
                                    }
                                }
                            } else if let Some(prop_name) = pred.strip_prefix("urn:synapse:prop:") {
                                if let Term::Literal(lit) = q.object {
                                    properties.insert(prop_name.to_string(), lit.value().to_string());
                                }
                            }
                        }

                        if !properties.is_empty() {
                            if let Err(e) = MarkdownWriter::update_frontmatter(&file_path, properties) {
                                eprintln!("Warning: Failed to update markdown frontmatter {}: {}", file_path, e);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Expand graph from a starting URI
    pub fn expand_graph(&self, start_uri: &str, depth: u32) -> Result<Vec<String>> {
        let mut expanded = Vec::new();

        if depth == 0 {
            return Ok(expanded);
        }

        // Query for all triples where start_uri is subject or object
        let subject = NamedNodeRef::new(start_uri).ok();

        if let Some(subj) = subject {
            for q in self
                .store
                .quads_for_pattern(Some(subj.into()), None, None, None)
                .flatten()
            {
                expanded.push(q.object.to_string());

                // Recursive expansion (simplified, depth-1)
                if depth > 1 {
                    let nested = self.expand_graph(&q.object.to_string(), depth - 1)?;
                    expanded.extend(nested);
                }
            }
        }

        Ok(expanded)
    }

    pub fn query_sparql(&self, query: &str) -> Result<String> {
        use oxigraph::sparql::QueryResults;

        let results = self.store.query(query)?;

        match results {
            QueryResults::Solutions(solutions) => {
                let mut results_array = Vec::new();
                for solution in solutions {
                    let sol = solution?;
                    let mut mapping = serde_json::Map::new();
                    for (variable, value) in sol.iter() {
                        mapping.insert(
                            variable.to_string(),
                            serde_json::to_value(value.to_string()).unwrap(),
                        );
                    }
                    results_array.push(serde_json::Value::Object(mapping));
                }
                Ok(serde_json::to_string(&results_array)?)
            }
            _ => Ok("[]".to_string()),
        }
    }

    pub fn ensure_uri(&self, s: &str) -> String {
        let clean = s.trim_start_matches('<').trim_end_matches('>');
        if clean.starts_with("http") || clean.starts_with("urn:") {
            clean.to_string()
        } else {
            format!("http://synapse.os/{}", clean)
        }
    }
}
