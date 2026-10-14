pub mod ontology;

use crate::iri;
use crate::md_sync::parser::MarkdownDocument;
use crate::store::{GraphMeta, SynapseStore, KIND_SOURCE};
use anyhow::{anyhow, Result};
use oxigraph::model::{Literal, NamedNode, Term, Triple};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Default)]
pub struct IndexReport {
    pub changed: usize,
    pub unchanged: usize,
    /// Sources indexed from this root before whose files no longer exist.
    pub removed: usize,
    pub triples: usize,
    pub errors: Vec<String>,
}

enum FileOutcome {
    Unchanged,
    Changed { triples: usize, affected: HashSet<NamedNode> },
}

pub struct IngestionEngine {
    store: std::sync::Arc<SynapseStore>,
}

impl IngestionEngine {
    pub fn new(store: std::sync::Arc<SynapseStore>) -> Self {
        Self { store }
    }

    /// Indexes every Markdown file under `dir`. Each file owns one named graph
    /// that is replaced when the file changes and dropped when it disappears;
    /// entity ids are the file paths relative to `dir` (OKF `concept_id`s).
    pub async fn index_markdown_directory(&self, dir: &Path) -> Result<IndexReport> {
        let root = dir.canonicalize().map_err(|e| anyhow!("cannot open {}: {e}", dir.display()))?;
        let root_str = root.to_string_lossy().to_string();
        let mut report = IndexReport::default();
        let mut seen = HashSet::new();
        let mut affected = HashSet::new();

        for entry in WalkDir::new(&root).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() || !is_markdown(path) {
                continue;
            }
            seen.insert(iri::source_graph(&path.to_string_lossy()));
            match self.ingest_markdown(&root, path, None) {
                Ok(FileOutcome::Unchanged) => report.unchanged += 1,
                Ok(FileOutcome::Changed { triples, affected: a }) => {
                    report.changed += 1;
                    report.triples += triples;
                    affected.extend(a);
                }
                Err(e) => report.errors.push(format!("{}: {e}", path.display())),
            }
        }

        for graph in self.store.graphs_indexed_from(&root_str) {
            if !seen.contains(&graph) {
                affected.extend(self.entities_in_graph(&graph));
                self.store.drop_graph(&graph)?;
                report.removed += 1;
            }
        }

        self.store.sync_entities_to_markdown(&affected);
        if report.changed + report.removed > 0 {
            self.digest();
        }
        Ok(report)
    }

    /// Ingests a single file. A Markdown file gets the id of its file stem
    /// (its parent directory acts as the root).
    pub async fn ingest_file(&self, path: &Path, role: Option<&str>) -> Result<usize> {
        let path = path.canonicalize().map_err(|e| anyhow!("cannot open {}: {e}", path.display()))?;
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

        let count = match extension.as_str() {
            "md" | "markdown" => {
                let root = path.parent().unwrap_or(Path::new("/")).to_path_buf();
                match self.ingest_markdown(&root, &path, role)? {
                    FileOutcome::Unchanged => 0,
                    FileOutcome::Changed { triples, affected } => {
                        self.store.sync_entities_to_markdown(&affected);
                        triples
                    }
                }
            }
            "csv" => self.ingest_csv(&path)?,
            "owl" | "ttl" | "rdf" | "xml" | "nt" => ontology::OntologyLoader::load_file(&self.store, &path)?.triples,
            _ => return Err(anyhow!("Unsupported file type: {}", extension)),
        };
        self.digest();
        Ok(count)
    }

    fn ingest_markdown(&self, root: &Path, path: &Path, role: Option<&str>) -> Result<FileOutcome> {
        let content = std::fs::read_to_string(path)?;
        let path_str = path.to_string_lossy().to_string();
        let doc = MarkdownDocument::parse(&concept_id(root, path), path, &content)?;
        let graph = iri::source_graph(&path_str);

        let same_content = self.store.graph_meta_value(&graph, "contentHash").as_deref() == Some(doc.content_hash.as_str());
        let same_role = self.store.graph_meta_value(&graph, "role").as_deref() == role;
        if same_content && same_role {
            return Ok(FileOutcome::Unchanged);
        }

        let triples = doc.to_triples()?;
        let mut affected = self.entities_in_graph(&graph);
        affected.insert(doc.subject()?);
        affected.extend(triples.iter().filter_map(|t| match &t.object {
            Term::NamedNode(n) => Some(n.clone()),
            _ => None,
        }));

        let meta = GraphMeta {
            kind: KIND_SOURCE.to_string(),
            derived_from: Some(iri::file_iri(&path_str)),
            method: Some("markdown".to_string()),
            content_hash: Some(doc.content_hash.clone()),
            concept_id: Some(doc.concept_id.clone()),
            index_root: Some(root.to_string_lossy().to_string()),
            role: role.map(str::to_string),
            ..Default::default()
        };
        let triples_count = self.store.replace_graph(&graph, &triples, &meta)?;
        Ok(FileOutcome::Changed { triples: triples_count, affected })
    }

    fn ingest_csv(&self, path: &Path) -> Result<usize> {
        let mut reader = csv::Reader::from_path(path)?;
        let headers = reader.headers()?.clone();
        let filename = path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();

        let mut triples = Vec::new();
        for record in reader.records() {
            let record = record?;
            // First column is the row id.
            let Some(id) = record.get(0).filter(|id| !id.is_empty()) else { continue };
            let subject = iri::entity(&format!("csv/{filename}/{id}"))?;
            for (j, field) in record.iter().enumerate().skip(1) {
                if let (Some(header), false) = (headers.get(j), field.is_empty()) {
                    triples.push(Triple::new(subject.clone(), iri::predicate(header)?, Literal::new_simple_literal(field)));
                }
            }
        }

        let path_str = path.to_string_lossy().to_string();
        let meta = GraphMeta {
            kind: KIND_SOURCE.to_string(),
            derived_from: Some(iri::file_iri(&path_str)),
            method: Some("csv".to_string()),
            ..Default::default()
        };
        self.store.replace_graph(&iri::source_graph(&path_str), &triples, &meta)
    }

    fn entities_in_graph(&self, graph: &NamedNode) -> HashSet<NamedNode> {
        self.store
            .store
            .quads_for_pattern(None, None, None, Some(graph.as_ref().into()))
            .flatten()
            .flat_map(|q| {
                let s = match q.subject {
                    oxigraph::model::Subject::NamedNode(n) => Some(n),
                    _ => None,
                };
                let o = match q.object {
                    Term::NamedNode(n) => Some(n),
                    _ => None,
                };
                s.into_iter().chain(o)
            })
            .collect()
    }

    fn digest(&self) {
        if let Err(e) = self.generate_cache_digest() {
            eprintln!("Warning: Failed to generate cache digest: {}", e);
        }
    }

    fn generate_cache_digest(&self) -> Result<()> {
        let query = "
            PREFIX prov: <http://www.w3.org/ns/prov#>

            SELECT ?s ?time (COUNT(?p) as ?density)
            WHERE {
                GRAPH ?g {
                    ?s ?p ?o .
                }
                ?g prov:generatedAtTime ?time .
            }
            GROUP BY ?s ?time
            ORDER BY DESC(?time) DESC(?density)
            LIMIT 5
        ";

        let result = self.store.query_sparql(query)?;
        let mut content = String::from("# Synapse Semantic Cache Digest\n\n_Auto-generated \"Morning Briefing\" of recent hot paths._\n\n## Most Recently Modified Entities:\n");

        match serde_json::from_str::<Vec<serde_json::Value>>(&result) {
            Ok(arr) if arr.is_empty() => content.push_str("No recent activity found.\n"),
            Ok(arr) => {
                for row in arr {
                    let get = |k: &str| row.get(k).and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
                    content.push_str(&format!("* **{}** (Edges: {}, Last Modified: {})\n", get("s"), get("density"), get("time")));
                }
            }
            Err(_) => content.push_str("Error parsing SPARQL results.\n"),
        }

        let synapse_dir = Path::new(".synapse/state");
        std::fs::create_dir_all(synapse_dir)?;
        std::fs::write(synapse_dir.join("current_digest.md"), content)?;
        Ok(())
    }
}

fn is_markdown(path: &Path) -> bool {
    matches!(path.extension().and_then(|e| e.to_str()), Some("md" | "markdown"))
}

/// `root/people/bob.md` -> `people/bob`.
pub fn concept_id(root: &Path, path: &Path) -> String {
    let rel: PathBuf = path.strip_prefix(root).unwrap_or(path).with_extension("");
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concept_ids_are_relative_paths_without_extension() {
        assert_eq!(concept_id(Path::new("/kb"), Path::new("/kb/people/bob.md")), "people/bob");
        assert_eq!(concept_id(Path::new("/kb"), Path::new("/kb/Bob.markdown")), "Bob");
    }
}
