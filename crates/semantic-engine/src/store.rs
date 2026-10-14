use crate::iri::{self, META_GRAPH};
use anyhow::{anyhow, Result};
use oxigraph::model::*;
use oxigraph::sparql::{Query, QueryResults};
use oxigraph::store::{StorageError, Store};
use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;
use uuid::Uuid;

/// Provenance of a named graph, stored in [`META_GRAPH`].
///
/// Every triple lives in exactly one named graph owned by its origin: a source
/// (a Markdown file, an ontology, a CSV) or an assertion (one `assert_facts`
/// call). Re-ingesting a source replaces its graph, so editing or deleting a
/// file retracts what it used to say.
#[derive(Debug, Clone, Default)]
pub struct GraphMeta {
    /// `Source` or `Assertion`.
    pub kind: String,
    pub derived_from: Option<NamedNode>,
    pub attributed_to: Option<String>,
    pub method: Option<String>,
    pub content_hash: Option<String>,
    pub concept_id: Option<String>,
    pub index_root: Option<String>,
    pub role: Option<String>,
    pub comment: Option<String>,
}

pub const KIND_SOURCE: &str = "Source";
pub const KIND_ASSERTION: &str = "Assertion";

#[derive(Debug)]
pub struct AssertOutcome {
    pub assertion: NamedNode,
    pub inserted: usize,
    /// Facts that were already supported by another graph (they are recorded
    /// anyway, as an extra source of support).
    pub already_known: usize,
}

#[derive(Debug, Default)]
pub struct RetractOutcome {
    pub retraction: Option<NamedNode>,
    pub removed: Vec<(Triple, NamedNode)>,
    pub not_found: Vec<Triple>,
    /// Facts that are stated by a source (e.g. a Markdown file): they can only
    /// be retracted by editing that source and re-indexing it.
    pub owned_by_sources: Vec<(Triple, NamedNode)>,
}

pub struct SynapseStore {
    pub store: Store,
    pub namespace: String,
    pub storage_path: Option<PathBuf>,
}

impl SynapseStore {
    pub fn open(namespace: &str, storage_path: &str) -> Result<Self> {
        let path = PathBuf::from(storage_path).join(namespace);
        std::fs::create_dir_all(&path)?;
        Ok(Self {
            store: Store::open(&path)?,
            namespace: namespace.to_string(),
            storage_path: Some(path),
        })
    }

    /// Non-persistent store, for tests and ephemeral sessions.
    pub fn in_memory() -> Result<Self> {
        Ok(Self {
            store: Store::new()?,
            namespace: "memory".to_string(),
            storage_path: None,
        })
    }

    /// Atomically replaces the contents and metadata of `graph`.
    pub fn replace_graph(&self, graph: &NamedNode, triples: &[Triple], meta: &GraphMeta) -> Result<usize> {
        let meta_quads = meta_quads(graph, meta);
        let inserted = self.store.transaction(|mut t| {
            clear_graph_and_meta(&mut t, graph)?;
            let mut inserted = 0;
            for tr in triples {
                if t.insert(QuadRef::new(&tr.subject, &tr.predicate, &tr.object, graph))? {
                    inserted += 1;
                }
            }
            for q in &meta_quads {
                t.insert(q)?;
            }
            Ok::<_, StorageError>(inserted)
        })?;
        Ok(inserted)
    }

    /// Removes a graph and its metadata. Returns the number of triples removed.
    pub fn drop_graph(&self, graph: &NamedNode) -> Result<usize> {
        Ok(self.store.transaction(|mut t| clear_graph_and_meta(&mut t, graph))?)
    }

    pub fn graph_meta_value(&self, graph: &NamedNode, term: &str) -> Option<String> {
        let meta = iri::node(META_GRAPH);
        let pred = iri::vocab(term);
        self.store
            .quads_for_pattern(Some(graph.as_ref().into()), Some(pred.as_ref()), None, Some(meta.as_ref().into()))
            .flatten()
            .find_map(|q| match q.object {
                Term::Literal(l) => Some(l.value().to_string()),
                _ => None,
            })
    }

    pub fn graph_kind(&self, graph: &NamedNode) -> Option<String> {
        let meta = iri::node(META_GRAPH);
        let rdf_type = iri::node(iri::RDF_TYPE);
        self.store
            .quads_for_pattern(Some(graph.as_ref().into()), Some(rdf_type.as_ref()), None, Some(meta.as_ref().into()))
            .flatten()
            .find_map(|q| match q.object {
                Term::NamedNode(n) => n.as_str().strip_prefix(iri::VOCAB_NS).map(str::to_string),
                _ => None,
            })
    }

    /// Source graphs whose metadata says they were indexed from `root`.
    pub fn graphs_indexed_from(&self, root: &str) -> Vec<NamedNode> {
        let meta = iri::node(META_GRAPH);
        let pred = iri::vocab("indexRoot");
        let root_lit = Literal::new_simple_literal(root);
        self.store
            .quads_for_pattern(None, Some(pred.as_ref()), Some(root_lit.as_ref().into()), Some(meta.as_ref().into()))
            .flatten()
            .filter_map(|q| match q.subject {
                Subject::NamedNode(n) => Some(n),
                _ => None,
            })
            .collect()
    }

    /// Records `triples` as one assertion, in a fresh named graph with provenance.
    pub fn assert_facts(&self, triples: &[Triple], source: &str, reason: Option<&str>) -> Result<AssertOutcome> {
        if triples.is_empty() {
            return Err(anyhow!("no facts to assert"));
        }
        let already_known = triples
            .iter()
            .filter(|t| {
                self.store
                    .quads_for_pattern(Some(t.subject.as_ref()), Some(t.predicate.as_ref()), Some(t.object.as_ref()), None)
                    .flatten()
                    .any(|q| q.graph_name.as_ref() != GraphNameRef::NamedNode(iri::node(META_GRAPH).as_ref()))
            })
            .count();
        let assertion = iri::node(&format!("{}{}", iri::ASSERTION_NS, Uuid::new_v4()));
        let meta = GraphMeta {
            kind: KIND_ASSERTION.to_string(),
            attributed_to: Some(source.to_string()),
            method: Some("assert_facts".to_string()),
            comment: reason.map(str::to_string),
            ..Default::default()
        };
        let inserted = self.replace_graph(&assertion, triples, &meta)?;
        self.sync_entities_to_markdown(&objects_of(triples));
        Ok(AssertOutcome { assertion, inserted, already_known })
    }

    /// Retracts a whole assertion. Sources cannot be retracted this way.
    pub fn retract_assertion(&self, assertion: &NamedNode, source: &str, reason: Option<&str>) -> Result<RetractOutcome> {
        match self.graph_kind(assertion).as_deref() {
            Some(KIND_ASSERTION) => {}
            Some(other) => return Err(anyhow!("{assertion} is a {other}, not an assertion: edit the source and re-index it instead")),
            None => return Err(anyhow!("unknown assertion {assertion}")),
        }
        let triples: Vec<Triple> = self
            .store
            .quads_for_pattern(None, None, None, Some(assertion.as_ref().into()))
            .flatten()
            .map(|q| Triple::new(q.subject, q.predicate, q.object))
            .collect();
        let removed = triples.into_iter().map(|t| (t, assertion.clone())).collect();
        self.apply_retraction(removed, Vec::new(), Vec::new(), source, reason)
    }

    /// Retracts individual facts from every assertion that states them.
    pub fn retract_facts(&self, triples: &[Triple], source: &str, reason: Option<&str>) -> Result<RetractOutcome> {
        let meta = iri::node(META_GRAPH);
        let mut removed = Vec::new();
        let mut owned = Vec::new();
        let mut not_found = Vec::new();
        for t in triples {
            let graphs: Vec<NamedNode> = self
                .store
                .quads_for_pattern(Some(t.subject.as_ref()), Some(t.predicate.as_ref()), Some(t.object.as_ref()), None)
                .flatten()
                .filter_map(|q| match q.graph_name {
                    GraphName::NamedNode(g) if g != meta => Some(g),
                    _ => None,
                })
                .collect();
            if graphs.is_empty() {
                not_found.push(t.clone());
            }
            for g in graphs {
                if self.graph_kind(&g).as_deref() == Some(KIND_ASSERTION) {
                    removed.push((t.clone(), g));
                } else {
                    owned.push((t.clone(), g));
                }
            }
        }
        self.apply_retraction(removed, owned, not_found, source, reason)
    }

    fn apply_retraction(
        &self,
        removed: Vec<(Triple, NamedNode)>,
        owned_by_sources: Vec<(Triple, NamedNode)>,
        not_found: Vec<Triple>,
        source: &str,
        reason: Option<&str>,
    ) -> Result<RetractOutcome> {
        if removed.is_empty() {
            return Ok(RetractOutcome { retraction: None, removed, not_found, owned_by_sources });
        }
        let meta = iri::node(META_GRAPH);
        let retraction = iri::node(&format!("{}{}", iri::RETRACTION_NS, Uuid::new_v4()));
        let now = now_literal();
        let mut log = vec![
            Quad::new(retraction.clone(), iri::node(iri::RDF_TYPE), iri::vocab("Retraction"), meta.clone()),
            Quad::new(retraction.clone(), iri::node(iri::PROV_GENERATED_AT), now.clone(), meta.clone()),
            Quad::new(retraction.clone(), iri::node(iri::PROV_ATTRIBUTED_TO), Literal::new_simple_literal(source), meta.clone()),
        ];
        if let Some(r) = reason {
            log.push(Quad::new(retraction.clone(), iri::node(iri::RDFS_COMMENT), Literal::new_simple_literal(r), meta.clone()));
        }
        let touched: BTreeSet<NamedNode> = removed.iter().map(|(_, g)| g.clone()).collect();
        for (t, g) in &removed {
            log.push(Quad::new(retraction.clone(), iri::vocab("retractedFact"), Literal::new_simple_literal(format!("{t} .")), meta.clone()));
            log.push(Quad::new(retraction.clone(), iri::vocab("fromGraph"), g.clone(), meta.clone()));
        }

        self.store.transaction(|mut t| {
            for (tr, g) in &removed {
                t.remove(QuadRef::new(&tr.subject, &tr.predicate, &tr.object, g))?;
            }
            for g in &touched {
                let empty = t.quads_for_pattern(None, None, None, Some(g.as_ref().into())).next().is_none();
                if empty {
                    t.insert(QuadRef::new(g, iri::node(iri::PROV_INVALIDATED_AT).as_ref(), now.as_ref(), &meta))?;
                }
            }
            for q in &log {
                t.insert(q)?;
            }
            Ok::<_, StorageError>(())
        })?;

        let triples: Vec<Triple> = removed.iter().map(|(t, _)| t.clone()).collect();
        self.sync_entities_to_markdown(&objects_of(&triples));
        Ok(RetractOutcome { retraction: Some(retraction), removed, not_found, owned_by_sources })
    }

    /// Regenerates the "Synapse Backlinks" section of the Markdown files that
    /// back the given entities (inbound edges only; the section is ignored when
    /// the file is parsed again, so it never feeds back into the graph).
    pub fn sync_entities_to_markdown(&self, entities: &HashSet<NamedNode>) {
        use crate::md_sync::writer::MarkdownWriter;

        let derived = iri::node(iri::PROV_DERIVED_FROM);
        let meta = iri::node(META_GRAPH);
        for entity in entities {
            let origin = self
                .store
                .quads_for_pattern(Some(entity.as_ref().into()), Some(derived.as_ref()), None, None)
                .flatten()
                .find_map(|q| match q.object {
                    Term::NamedNode(n) => iri::file_path(n.as_str()),
                    _ => None,
                });
            let Some(path) = origin else { continue };
            if !std::path::Path::new(&path).exists() {
                continue;
            }
            let backlinks: BTreeSet<(String, String)> = self
                .store
                .quads_for_pattern(None, None, Some(entity.as_ref().into()), None)
                .flatten()
                .filter(|q| q.predicate != derived && q.graph_name != GraphName::NamedNode(meta.clone()))
                .filter_map(|q| match q.subject {
                    Subject::NamedNode(s) if &s != entity => Some((iri::short(q.predicate.as_str()), iri::short(s.as_str()))),
                    _ => None,
                })
                .collect();
            let backlinks: Vec<_> = backlinks.into_iter().collect();
            if let Err(e) = MarkdownWriter::write_backlinks(&path, &backlinks) {
                eprintln!("Warning: failed to sync markdown backlinks {}: {}", path, e);
            }
        }
    }

    /// Expand graph from a starting URI
    pub fn expand_graph(&self, start_uri: &str, depth: u32) -> Result<Vec<String>> {
        let mut expanded = Vec::new();
        if depth == 0 {
            return Ok(expanded);
        }
        if let Ok(subj) = NamedNodeRef::new(start_uri) {
            for q in self.store.quads_for_pattern(Some(subj.into()), None, None, None).flatten() {
                match q.object {
                    Term::NamedNode(n) => {
                        expanded.push(n.as_str().to_string());
                        if depth > 1 {
                            expanded.extend(self.expand_graph(n.as_str(), depth - 1)?);
                        }
                    }
                    other => expanded.push(other.to_string()),
                }
            }
        }
        Ok(expanded)
    }

    /// Runs a SPARQL query. Unless the query names its own dataset (FROM / FROM
    /// NAMED), the default graph is the union of all graphs, so facts are found
    /// whichever source or assertion they belong to.
    pub fn query_sparql(&self, query: &str) -> Result<String> {
        let mut query = Query::parse(query, None)?;
        if query.dataset().is_default_dataset() {
            query.dataset_mut().set_default_graph_as_union();
        }
        match self.store.query(query)? {
            QueryResults::Solutions(solutions) => {
                let mut results_array = Vec::new();
                for solution in solutions {
                    let sol = solution?;
                    let mut mapping = serde_json::Map::new();
                    for (variable, value) in sol.iter() {
                        mapping.insert(variable.as_str().to_string(), serde_json::Value::String(value.to_string()));
                    }
                    results_array.push(serde_json::Value::Object(mapping));
                }
                Ok(serde_json::to_string(&results_array)?)
            }
            QueryResults::Boolean(b) => Ok(b.to_string()),
            QueryResults::Graph(triples) => {
                let mut out = String::new();
                for t in triples {
                    out.push_str(&format!("{} .\n", t?));
                }
                Ok(out)
            }
        }
    }
}

fn clear_graph_and_meta(t: &mut oxigraph::store::Transaction<'_>, graph: &NamedNode) -> Result<usize, StorageError> {
    let meta = iri::node(META_GRAPH);
    let content: Vec<Quad> = t
        .quads_for_pattern(None, None, None, Some(graph.as_ref().into()))
        .collect::<Result<_, _>>()?;
    let meta_quads: Vec<Quad> = t
        .quads_for_pattern(Some(graph.as_ref().into()), None, None, Some(meta.as_ref().into()))
        .collect::<Result<_, _>>()?;
    for q in content.iter().chain(meta_quads.iter()) {
        t.remove(q)?;
    }
    Ok(content.len())
}

fn meta_quads(graph: &NamedNode, meta: &GraphMeta) -> Vec<Quad> {
    let g = iri::node(META_GRAPH);
    let mut out = vec![
        Quad::new(graph.clone(), iri::node(iri::RDF_TYPE), iri::vocab(&meta.kind), g.clone()),
        Quad::new(graph.clone(), iri::node(iri::PROV_GENERATED_AT), now_literal(), g.clone()),
    ];
    if let Some(d) = &meta.derived_from {
        out.push(Quad::new(graph.clone(), iri::node(iri::PROV_DERIVED_FROM), d.clone(), g.clone()));
    }
    if let Some(a) = &meta.attributed_to {
        out.push(Quad::new(graph.clone(), iri::node(iri::PROV_ATTRIBUTED_TO), Literal::new_simple_literal(a), g.clone()));
    }
    if let Some(c) = &meta.comment {
        out.push(Quad::new(graph.clone(), iri::node(iri::RDFS_COMMENT), Literal::new_simple_literal(c), g.clone()));
    }
    for (term, value) in [
        ("method", &meta.method),
        ("contentHash", &meta.content_hash),
        ("conceptId", &meta.concept_id),
        ("indexRoot", &meta.index_root),
        ("role", &meta.role),
    ] {
        if let Some(v) = value {
            out.push(Quad::new(graph.clone(), iri::vocab(term), Literal::new_simple_literal(v), g.clone()));
        }
    }
    out
}

fn now_literal() -> Literal {
    Literal::new_typed_literal(chrono::Utc::now().to_rfc3339(), iri::node(iri::XSD_DATE_TIME))
}

fn objects_of(triples: &[Triple]) -> HashSet<NamedNode> {
    triples
        .iter()
        .filter_map(|t| match &t.object {
            Term::NamedNode(n) => Some(n.clone()),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fact(s: &str, p: &str, o: &str) -> Triple {
        Triple::new(iri::entity(s).unwrap(), iri::predicate(p).unwrap(), iri::entity(o).unwrap())
    }

    fn count(store: &SynapseStore, q: &str) -> usize {
        serde_json::from_str::<Vec<serde_json::Value>>(&store.query_sparql(q).unwrap()).unwrap().len()
    }

    #[test]
    fn replace_graph_retracts_what_the_source_no_longer_says() {
        let store = SynapseStore::in_memory().unwrap();
        let g = iri::node("urn:synapse:source:test");
        let meta = GraphMeta { kind: KIND_SOURCE.into(), ..Default::default() };
        store.replace_graph(&g, &[fact("a", "knows", "b"), fact("a", "knows", "c")], &meta).unwrap();
        store.replace_graph(&g, &[fact("a", "knows", "c")], &meta).unwrap();
        let q = "SELECT ?o WHERE { <urn:synapse:entity:a> <urn:synapse:prop:knows> ?o }";
        assert_eq!(count(&store, q), 1);
    }

    #[test]
    fn queries_see_every_named_graph_by_default() {
        let store = SynapseStore::in_memory().unwrap();
        store.assert_facts(&[fact("a", "knows", "b")], "test", None).unwrap();
        assert_eq!(count(&store, "SELECT * WHERE { ?s <urn:synapse:prop:knows> ?o }"), 1);
        // An explicit dataset is respected.
        assert_eq!(count(&store, "SELECT * FROM <urn:nothing> WHERE { ?s <urn:synapse:prop:knows> ?o }"), 0);
    }

    #[test]
    fn assertion_can_be_retracted_whole_and_is_logged() {
        let store = SynapseStore::in_memory().unwrap();
        let out = store.assert_facts(&[fact("a", "knows", "b")], "agent", Some("met at conf")).unwrap();
        assert_eq!(out.inserted, 1);
        let r = store.retract_assertion(&out.assertion, "agent", Some("wrong person")).unwrap();
        assert_eq!(r.removed.len(), 1);
        assert_eq!(count(&store, "SELECT * WHERE { ?s <urn:synapse:prop:knows> ?o }"), 0);
        assert_eq!(count(&store, "SELECT * WHERE { ?r <urn:synapse:vocab:retractedFact> ?f }"), 1);
    }

    #[test]
    fn retract_facts_only_touches_assertions() {
        let store = SynapseStore::in_memory().unwrap();
        let src = iri::node("urn:synapse:source:file:/x.md");
        store
            .replace_graph(&src, &[fact("a", "knows", "b")], &GraphMeta { kind: KIND_SOURCE.into(), ..Default::default() })
            .unwrap();
        let out = store.assert_facts(&[fact("a", "knows", "b")], "agent", None).unwrap();
        assert_eq!(out.already_known, 1);

        let r = store.retract_facts(&[fact("a", "knows", "b"), fact("a", "knows", "zzz")], "agent", None).unwrap();
        assert_eq!(r.removed.len(), 1);
        assert_eq!(r.owned_by_sources.len(), 1);
        assert_eq!(r.not_found.len(), 1);
        // Still supported by the source file.
        assert_eq!(count(&store, "SELECT * WHERE { ?s <urn:synapse:prop:knows> ?o }"), 1);
    }

    #[test]
    fn sources_cannot_be_retracted_as_assertions() {
        let store = SynapseStore::in_memory().unwrap();
        let src = iri::node("urn:synapse:source:file:/x.md");
        store
            .replace_graph(&src, &[fact("a", "knows", "b")], &GraphMeta { kind: KIND_SOURCE.into(), ..Default::default() })
            .unwrap();
        assert!(store.retract_assertion(&src, "agent", None).is_err());
    }
}
