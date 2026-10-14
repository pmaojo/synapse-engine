//! Identity rules shared by every entry point (Markdown, MCP tools, CSV).
//!
//! An entity is identified by a stable, human-readable id — the path of its
//! Markdown file relative to the indexed root, without extension (the OKF
//! `concept_id`), or whatever id an agent uses in `assert_facts`. The same id
//! always maps to the same IRI, so `[[people/bob]]`, `people/bob.md` and an
//! agent asserting about `people/bob` all talk about one node.

use anyhow::{anyhow, Result};
use oxigraph::model::NamedNode;

pub const ENTITY_NS: &str = "urn:synapse:entity:";
pub const PROP_NS: &str = "urn:synapse:prop:";
pub const TYPE_NS: &str = "urn:synapse:type:";
/// Named graph holding the triples extracted from one source (file, ontology, CSV).
pub const SOURCE_NS: &str = "urn:synapse:source:";
/// Named graph holding the facts of one `assert_facts` call.
pub const ASSERTION_NS: &str = "urn:synapse:assertion:";
pub const RETRACTION_NS: &str = "urn:synapse:retraction:";
/// Synapse's own bookkeeping vocabulary.
pub const VOCAB_NS: &str = "urn:synapse:vocab:";
/// Named graph with the metadata (provenance) of every other graph.
pub const META_GRAPH: &str = "urn:synapse:meta";

pub const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
pub const RDFS_LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";
pub const RDFS_COMMENT: &str = "http://www.w3.org/2000/01/rdf-schema#comment";
pub const PROV_DERIVED_FROM: &str = "http://www.w3.org/ns/prov#wasDerivedFrom";
pub const PROV_GENERATED_AT: &str = "http://www.w3.org/ns/prov#generatedAtTime";
pub const PROV_INVALIDATED_AT: &str = "http://www.w3.org/ns/prov#invalidatedAtTime";
pub const PROV_ATTRIBUTED_TO: &str = "http://www.w3.org/ns/prov#wasAttributedTo";
pub const XSD_DATE_TIME: &str = "http://www.w3.org/2001/XMLSchema#dateTime";

const PREFIXES: &[(&str, &str)] = &[
    ("rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#"),
    ("rdfs", "http://www.w3.org/2000/01/rdf-schema#"),
    ("owl", "http://www.w3.org/2002/07/owl#"),
    ("xsd", "http://www.w3.org/2001/XMLSchema#"),
    ("prov", "http://www.w3.org/ns/prov#"),
    ("schema", "http://schema.org/"),
    ("skos", "http://www.w3.org/2004/02/skos/core#"),
    ("foaf", "http://xmlns.com/foaf/0.1/"),
];

const IRI_SCHEMES: &[&str] = &["http", "https", "urn", "file", "mailto", "ui"];

pub fn vocab(term: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("{VOCAB_NS}{term}"))
}

pub fn node(iri: &str) -> NamedNode {
    NamedNode::new_unchecked(iri)
}

/// Resolves an entity id (`people/bob`, `Bob`, `schema:Person`, `https://…`).
pub fn entity(id: &str) -> Result<NamedNode> {
    resolve(id, ENTITY_NS)
}

/// Resolves a predicate (`depends_on`, `a`, `rdfs:label`, `https://…`).
pub fn predicate(id: &str) -> Result<NamedNode> {
    if id.trim() == "a" {
        return Ok(node(RDF_TYPE));
    }
    resolve(id, PROP_NS)
}

/// Resolves a class name used as the object of `rdf:type` (`Person`, `schema:Person`).
pub fn class(id: &str) -> Result<NamedNode> {
    resolve(id, TYPE_NS)
}

/// Resolves a datatype (`xsd:integer`, full IRI).
pub fn datatype(id: &str) -> Result<NamedNode> {
    let id = id.trim();
    expand_prefixed(id)
        .or_else(|| is_absolute(id).then(|| id.to_string()))
        .ok_or_else(|| anyhow!("unknown datatype '{id}': use a known prefix (xsd:…) or a full IRI"))
        .and_then(|iri| NamedNode::new(iri).map_err(|e| anyhow!("invalid datatype IRI: {e}")))
}

/// IRI of a local file, used as the `prov:wasDerivedFrom` target of Markdown entities.
pub fn file_iri(path: &str) -> NamedNode {
    node(&format!("file://{}", encode_path(path)))
}

/// Inverse of [`file_iri`].
pub fn file_path(iri: &str) -> Option<String> {
    let encoded = iri.strip_prefix("file://")?;
    urlencoding::decode(encoded).ok().map(|p| p.into_owned())
}

/// Named graph owning the triples extracted from a file.
pub fn source_graph(path: &str) -> NamedNode {
    node(&format!("{SOURCE_NS}file:{}", encode_path(path)))
}

/// Short form for display: strips the Synapse namespaces and known prefixes.
pub fn short(iri: &str) -> String {
    for ns in [ENTITY_NS, PROP_NS, TYPE_NS] {
        if let Some(rest) = iri.strip_prefix(ns) {
            return urlencoding::decode(rest).map(|s| s.into_owned()).unwrap_or_else(|_| rest.to_string());
        }
    }
    for (prefix, ns) in PREFIXES {
        if let Some(rest) = iri.strip_prefix(ns) {
            return format!("{prefix}:{rest}");
        }
    }
    iri.to_string()
}

fn resolve(id: &str, default_ns: &str) -> Result<NamedNode> {
    let id = id.trim().trim_start_matches('<').trim_end_matches('>').trim();
    if id.is_empty() {
        return Err(anyhow!("empty identifier"));
    }
    let iri = if let Some(expanded) = expand_prefixed(id) {
        expanded
    } else if is_absolute(id) {
        id.to_string()
    } else {
        format!("{default_ns}{}", encode_local(id))
    };
    NamedNode::new(iri).map_err(|e| anyhow!("invalid identifier '{id}': {e}"))
}

fn expand_prefixed(id: &str) -> Option<String> {
    let (prefix, local) = id.split_once(':')?;
    PREFIXES
        .iter()
        .find(|(p, _)| *p == prefix)
        .map(|(_, ns)| format!("{ns}{local}"))
}

/// True for ids that already are IRIs (`https://…`, `urn:…`), never for plain names.
pub fn is_absolute(id: &str) -> bool {
    match id.split_once(':') {
        Some((scheme, _)) => IRI_SCHEMES.contains(&scheme.to_ascii_lowercase().as_str()),
        None => false,
    }
}

/// True when `prefix` is a known CURIE prefix or IRI scheme rather than a relation name.
pub fn is_reserved_prefix(prefix: &str) -> bool {
    let p = prefix.to_ascii_lowercase();
    IRI_SCHEMES.contains(&p.as_str()) || PREFIXES.iter().any(|(k, _)| *k == p)
}

/// Spaces become `_` (wiki convention); characters not allowed in an IRI are percent-encoded.
fn encode_local(id: &str) -> String {
    let mut out = String::with_capacity(id.len());
    for c in id.chars() {
        match c {
            ' ' => out.push('_'),
            c if c.is_control() || "<>\"{}|\\^`#?".contains(c) => push_encoded(&mut out, c),
            c => out.push(c),
        }
    }
    out
}

fn encode_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for c in path.chars() {
        if c.is_control() || c == ' ' || "<>\"{}|\\^`#?%".contains(c) {
            push_encoded(&mut out, c);
        } else {
            out.push(c);
        }
    }
    out
}

fn push_encoded(out: &mut String, c: char) {
    let mut buf = [0u8; 4];
    for b in c.encode_utf8(&mut buf).bytes() {
        out.push_str(&format!("%{b:02X}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_ids_live_in_synapse_namespaces() {
        assert_eq!(entity("people/bob").unwrap().as_str(), "urn:synapse:entity:people/bob");
        assert_eq!(entity("Bob Smith").unwrap().as_str(), "urn:synapse:entity:Bob_Smith");
        assert_eq!(predicate("depends_on").unwrap().as_str(), "urn:synapse:prop:depends_on");
        assert_eq!(class("Person").unwrap().as_str(), "urn:synapse:type:Person");
    }

    #[test]
    fn curies_and_absolute_iris_are_kept() {
        assert_eq!(predicate("a").unwrap().as_str(), RDF_TYPE);
        assert_eq!(predicate("rdfs:label").unwrap().as_str(), RDFS_LABEL);
        assert_eq!(class("schema:Person").unwrap().as_str(), "http://schema.org/Person");
        assert_eq!(entity("https://example.org/x").unwrap().as_str(), "https://example.org/x");
        assert_eq!(entity("<urn:foo:bar>").unwrap().as_str(), "urn:foo:bar");
    }

    #[test]
    fn hostile_input_cannot_break_out_of_an_iri() {
        let iri = entity("x> } ; DROP ALL ; <y").unwrap();
        assert!(!iri.as_str().contains('>'));
        assert!(!iri.as_str().contains(' '));
    }

    #[test]
    fn file_iris_round_trip() {
        let iri = file_iri("/tmp/my notes/a#b.md");
        assert_eq!(file_path(iri.as_str()).unwrap(), "/tmp/my notes/a#b.md");
    }

    #[test]
    fn short_strips_namespaces() {
        assert_eq!(short("urn:synapse:entity:people/bob"), "people/bob");
        assert_eq!(short(RDFS_LABEL), "rdfs:label");
    }
}
