use crate::iri;
use crate::store::{GraphMeta, SynapseStore, KIND_SOURCE};
use anyhow::{anyhow, Result};
use oxigraph::io::{RdfFormat, RdfParser};
use oxigraph::model::Triple;
use sha2::{Digest, Sha256};
use std::path::Path;

pub struct OntologyLoader;

pub struct LoadOutcome {
    pub triples: usize,
    /// False when the file was already loaded with the same content.
    pub changed: bool,
}

impl OntologyLoader {
    /// Loads an RDF file into its own named graph, replacing what a previous
    /// load of the same file put there. Unchanged files are skipped.
    pub fn load_file(store: &SynapseStore, path: &Path) -> Result<LoadOutcome> {
        let bytes = std::fs::read(path)?;
        let path_str = path.canonicalize().unwrap_or_else(|_| path.to_path_buf()).to_string_lossy().to_string();
        let graph = iri::source_graph(&path_str);
        let hash = hex::encode(Sha256::digest(&bytes));
        if store.graph_meta_value(&graph, "contentHash").as_deref() == Some(hash.as_str()) {
            return Ok(LoadOutcome { triples: 0, changed: false });
        }

        let format = match path.extension().and_then(|e| e.to_str()).map(str::to_lowercase).as_deref() {
            Some("rdf" | "owl" | "xml") => RdfFormat::RdfXml,
            Some("nt") => RdfFormat::NTriples,
            _ => RdfFormat::Turtle,
        };
        let triples = RdfParser::from_format(format)
            .rename_blank_nodes()
            .for_reader(bytes.as_slice())
            .map(|q| q.map(|q| Triple::new(q.subject, q.predicate, q.object)))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| anyhow!("Parse error in {}: {}", path.display(), e))?;

        let meta = GraphMeta {
            kind: KIND_SOURCE.to_string(),
            derived_from: Some(iri::file_iri(&path_str)),
            method: Some("ontology_loader".to_string()),
            content_hash: Some(hash),
            ..Default::default()
        };
        let triples = store.replace_graph(&graph, &triples, &meta)?;
        Ok(LoadOutcome { triples, changed: true })
    }
}
