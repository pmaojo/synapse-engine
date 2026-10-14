use crate::iri;
use anyhow::Result;
use oxigraph::model::{Literal, NamedNode, Term, Triple};
use regex::Regex;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::OnceLock;
use yaml_front_matter::{Document, YamlFrontMatter};

/// Heading of the section [`super::writer::MarkdownWriter`] maintains. Everything
/// from this heading on is ignored when parsing, so derived backlinks never
/// become facts of their own.
pub const BACKLINKS_HEADING: &str = "## 🧠 Synapse Backlinks";

#[derive(Debug, Deserialize, Clone, Default)]
pub struct SynapseFrontmatter {
    #[serde(rename = "type")]
    pub entity_type: Option<String>,
    /// Explicit identity; overrides the path-derived id.
    pub uri: Option<String>,
    #[serde(flatten)]
    pub properties: std::collections::BTreeMap<String, serde_yaml::Value>,
}

#[derive(Debug, PartialEq)]
pub struct SemanticLink {
    /// Relation of a typed link: `rel` in `[[rel:target]]` (OKF) or `[[rel::target]]`.
    pub property: Option<String>,
    pub target: String,
}

pub struct MarkdownDocument {
    /// Stable id: the path relative to the indexed root, without extension.
    pub concept_id: String,
    pub file_path: String,
    /// SHA-256 of the file without its backlinks section.
    pub content_hash: String,
    pub frontmatter: Option<SynapseFrontmatter>,
    pub links: Vec<SemanticLink>,
}

impl MarkdownDocument {
    pub fn parse(concept_id: &str, file_path: &Path, raw_content: &str) -> Result<Self> {
        let raw = strip_backlinks(raw_content);
        let mut hasher = Sha256::new();
        hasher.update(raw.as_bytes());
        let content_hash = hex::encode(hasher.finalize());

        let (frontmatter, body) = if raw.trim_start().starts_with("---") {
            match YamlFrontMatter::parse::<SynapseFrontmatter>(raw) {
                Ok(Document { metadata, content }) => (Some(metadata), content),
                Err(_) => (None, raw.to_string()),
            }
        } else {
            (None, raw.to_string())
        };

        Ok(Self {
            concept_id: concept_id.to_string(),
            file_path: file_path.to_string_lossy().to_string(),
            content_hash,
            frontmatter,
            links: extract_links(&body),
        })
    }

    /// The entity this document describes.
    pub fn subject(&self) -> Result<NamedNode> {
        match self.frontmatter.as_ref().and_then(|f| f.uri.as_deref()) {
            Some(uri) => iri::entity(uri),
            None => iri::entity(&self.concept_id),
        }
    }

    /// Triples stated by the document (the caller decides which graph owns them).
    pub fn to_triples(&self) -> Result<Vec<Triple>> {
        let subject = self.subject()?;
        let mut out = vec![Triple::new(
            subject.clone(),
            iri::node(iri::PROV_DERIVED_FROM),
            iri::file_iri(&self.file_path),
        )];

        if let Some(fm) = &self.frontmatter {
            if let Some(t) = &fm.entity_type {
                out.push(Triple::new(subject.clone(), iri::node(iri::RDF_TYPE), iri::class(t)?));
            }
            for (key, value) in &fm.properties {
                let pred = iri::predicate(key)?;
                for lit in yaml_literals(value) {
                    if key == "title" {
                        out.push(Triple::new(subject.clone(), iri::node(iri::RDFS_LABEL), lit.clone()));
                    }
                    out.push(Triple::new(subject.clone(), pred.clone(), lit));
                }
            }
        }

        for link in &self.links {
            let pred = match &link.property {
                Some(p) => iri::predicate(p)?,
                None => iri::predicate("mentions")?,
            };
            out.push(Triple::new(subject.clone(), pred, iri::entity(&link.target)?));
        }
        Ok(out)
    }
}

/// Content without the backlinks section nor trailing whitespace (so writing
/// the section never changes the content hash).
pub fn strip_backlinks(content: &str) -> &str {
    match content.find(BACKLINKS_HEADING) {
        Some(i) => &content[..i],
        None => content,
    }
    .trim_end()
}

fn extract_links(body: &str) -> Vec<SemanticLink> {
    static RE: OnceLock<Regex> = OnceLock::new();
    // [[target]], [[target|alias]], [[rel:target]], [[rel::target]]
    let re = RE.get_or_init(|| Regex::new(r"\[\[([^\[\]|]+?)(?:\|[^\]]*)?\]\]").unwrap());
    re.captures_iter(body).filter_map(|c| parse_link(c[1].trim())).collect()
}

fn parse_link(inner: &str) -> Option<SemanticLink> {
    if inner.is_empty() {
        return None;
    }
    if let Some((rel, target)) = inner.split_once("::") {
        return Some(SemanticLink { property: Some(rel.trim().to_string()), target: target.trim().to_string() });
    }
    if let Some((rel, target)) = inner.split_once(':') {
        let is_rel = !rel.is_empty()
            && rel.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            && !iri::is_reserved_prefix(rel)
            && !target.starts_with("//");
        if is_rel && !target.trim().is_empty() {
            return Some(SemanticLink { property: Some(rel.to_string()), target: target.trim().to_string() });
        }
    }
    Some(SemanticLink { property: None, target: inner.to_string() })
}

fn yaml_literals(value: &serde_yaml::Value) -> Vec<Term> {
    use serde_yaml::Value;
    let xsd = |t: &str| iri::node(&format!("http://www.w3.org/2001/XMLSchema#{t}"));
    match value {
        Value::String(s) => vec![Literal::new_simple_literal(s).into()],
        Value::Bool(b) => vec![Literal::new_typed_literal(b.to_string(), xsd("boolean")).into()],
        Value::Number(n) if n.is_f64() => vec![Literal::new_typed_literal(n.to_string(), xsd("double")).into()],
        Value::Number(n) => vec![Literal::new_typed_literal(n.to_string(), xsd("integer")).into()],
        Value::Sequence(items) => items.iter().flat_map(yaml_literals).collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(raw: &str) -> MarkdownDocument {
        MarkdownDocument::parse("people/alice", Path::new("/kb/people/alice.md"), raw).unwrap()
    }

    #[test]
    fn identity_comes_from_the_path_not_the_content() {
        let a = parse("---\ntype: person\n---\nHello");
        let b = parse("---\ntype: person\n---\nHello, edited");
        assert_eq!(a.subject().unwrap(), b.subject().unwrap());
        assert_eq!(a.subject().unwrap().as_str(), "urn:synapse:entity:people/alice");
        assert_ne!(a.content_hash, b.content_hash);
    }

    #[test]
    fn frontmatter_uri_overrides_identity() {
        let d = parse("---\nuri: https://example.org/alice\n---\n");
        assert_eq!(d.subject().unwrap().as_str(), "https://example.org/alice");
    }

    #[test]
    fn okf_and_legacy_links() {
        let d = parse("Works with [[people/bob]], [[depends_on:libs/sqlx]], [[es_un::Concepto]], [[Bob|el bueno de Bob]], [[https://x.org/a]] and [[schema:Person]].");
        let links: Vec<_> = d.links.iter().map(|l| (l.property.as_deref(), l.target.as_str())).collect();
        assert_eq!(
            links,
            vec![
                (None, "people/bob"),
                (Some("depends_on"), "libs/sqlx"),
                (Some("es_un"), "Concepto"),
                (None, "Bob"),
                (None, "https://x.org/a"),
                (None, "schema:Person"),
            ]
        );
    }

    #[test]
    fn backlinks_section_is_ignored() {
        let d = parse("Body [[a]]\n\n## 🧠 Synapse Backlinks\n- **mentions**: [[b]]\n");
        assert_eq!(d.links.len(), 1);
        assert_eq!(d.content_hash, parse("Body [[a]]\n").content_hash);
    }

    #[test]
    fn frontmatter_values_become_typed_literals() {
        let d = parse("---\ntype: person\ntitle: Alice\nage: 30\ntags:\n  - staff\n  - rust\n---\n");
        let t = d.to_triples().unwrap();
        let has = |p: &str, o: &str| t.iter().any(|tr| tr.predicate.as_str() == p && tr.object.to_string() == o);
        assert!(has(iri::RDF_TYPE, "<urn:synapse:type:person>"));
        assert!(has(iri::RDFS_LABEL, "\"Alice\""));
        assert!(has("urn:synapse:prop:age", "\"30\"^^<http://www.w3.org/2001/XMLSchema#integer>"));
        assert!(has("urn:synapse:prop:tags", "\"rust\""));
    }
}
