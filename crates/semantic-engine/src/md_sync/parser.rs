use oxrdf::{NamedNode, Quad, Subject, Term, Literal};
use regex::Regex;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::Path;
use yaml_front_matter::{Document, YamlFrontMatter};

#[derive(Debug, Deserialize, Clone)]
pub struct SynapseFrontmatter {
    #[serde(rename = "type")]
    pub entity_type: Option<String>,
    pub uri: Option<String>,
    // Permite campos extra dinámicos
    #[serde(flatten)]
    pub properties: std::collections::HashMap<String, serde_yaml::Value>,
}

pub struct ParsedBlock {
    pub hash: String,
    pub content: String,
    pub extracted_links: Vec<SemanticLink>,
}

#[derive(Debug)]
pub struct SemanticLink {
    pub property: Option<String>, // e.g., "es_un" from [[es_un::Concepto]]
    pub target: String,           // e.g., "Concepto"
}

pub struct MarkdownDocument {
    pub file_path: String,
    pub file_hash: String,
    pub frontmatter: Option<SynapseFrontmatter>,
    pub blocks: Vec<ParsedBlock>,
}

impl MarkdownDocument {
    pub fn parse<P: AsRef<Path>>(path: P, raw_content: &str) -> Result<Self, anyhow::Error> {
        // 1. Extraer YAML Frontmatter
        let document: Result<Document<SynapseFrontmatter>, _> = YamlFrontMatter::parse(raw_content);

        let (metadata, content) = match document {
            Ok(doc) => (Some(doc.metadata), doc.content),
            Err(_) => (None, raw_content.to_string()),
        };

        // 2. Hashear el archivo para proveniencia
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let file_hash = hex::encode(hasher.finalize());

        // 3. Parsear bloques e extraer wikilinks usando RegEx
        // ([[Entidad]] o [[propiedad::Entidad]])
        let mut blocks = Vec::new();
        let wikilink_re = Regex::new(r"\[\[(?:(?P<prop>[^\]:]+)::)?(?P<target>[^\]]+)\]\]").unwrap();

        // Tratamos cada línea o párrafo separado como un bloque lógico simple por ahora
        for (i, p) in content.split("\n\n").enumerate() {
            let p_trim = p.trim();
            if p_trim.is_empty() {
                continue;
            }

            let mut block_hasher = Sha256::new();
            block_hasher.update(p_trim.as_bytes());
            block_hasher.update(file_hash.as_bytes());
            block_hasher.update(i.to_string().as_bytes());
            let block_hash = hex::encode(block_hasher.finalize());

            let mut extracted_links = Vec::new();
            for caps in wikilink_re.captures_iter(p_trim) {
                let prop = caps.name("prop").map(|m| m.as_str().to_string());
                let target = caps.name("target").unwrap().as_str().to_string();
                extracted_links.push(SemanticLink { property: prop, target });
            }

            blocks.push(ParsedBlock {
                hash: block_hash,
                content: p_trim.to_string(),
                extracted_links,
            });
        }

        Ok(Self {
            file_path: path.as_ref().to_string_lossy().to_string(),
            file_hash,
            frontmatter: metadata,
            blocks,
        })
    }

    /// Genera los N-Quads listos para insertar en Oxigraph
    pub fn to_quads(&self) -> Vec<Quad> {
        let mut quads = Vec::new();
        let base_uri = self
            .frontmatter
            .as_ref()
            .and_then(|f| f.uri.clone())
            .unwrap_or_else(|| format!("urn:synapse:entity:{}", self.file_hash));

        let subject_node = NamedNode::new(base_uri.clone()).unwrap();
        let subject = Subject::NamedNode(subject_node.clone());

        // Identificador del archivo de origen
        let file_node = NamedNode::new(format!("file://{}", self.file_path)).unwrap();

        // 1. Tripleta base: El archivo MD representa esta entidad
        let p_derived = NamedNode::new("http://www.w3.org/ns/prov#wasDerivedFrom").unwrap();
        quads.push(Quad::new(
            subject.clone(),
            p_derived.clone(),
            Term::NamedNode(file_node),
            oxrdf::GraphName::DefaultGraph,
        ));

        // 2. Propiedades del Frontmatter
        if let Some(fm) = &self.frontmatter {
            if let Some(t) = &fm.entity_type {
                let p_type = NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type").unwrap();
                let o_type = NamedNode::new(format!("urn:synapse:type:{}", t)).unwrap();
                quads.push(Quad::new(
                    subject.clone(),
                    p_type,
                    Term::NamedNode(o_type),
                    oxrdf::GraphName::DefaultGraph,
                ));
            }

            for (k, v) in &fm.properties {
                let pred = NamedNode::new(format!("urn:synapse:prop:{}", k)).unwrap();

                let value_str = match v {
                    serde_yaml::Value::String(s) => s.clone(),
                    serde_yaml::Value::Number(n) => n.to_string(),
                    serde_yaml::Value::Bool(b) => b.to_string(),
                    _ => continue, // Skip complex nested types for now
                };

                quads.push(Quad::new(
                    subject.clone(),
                    pred,
                    Term::Literal(Literal::new_simple_literal(value_str)),
                    oxrdf::GraphName::DefaultGraph,
                ));
            }
        }

        // 3. Relaciones extraídas de los bloques (Wikilinks)
        for block in &self.blocks {
            let block_uri = format!("urn:synapse:md:{}#block_{}", self.file_hash, block.hash);
            let block_node = NamedNode::new(block_uri.clone()).unwrap();

            for link in &block.extracted_links {
                let pred_uri = if let Some(p) = &link.property {
                    format!("urn:synapse:prop:{}", p)
                } else {
                    "urn:synapse:prop:mentions".to_string()
                };

                let pred = NamedNode::new(pred_uri).unwrap();

                // Tratar el target como un URN si no es URL
                let target_uri = if link.target.starts_with("http") || link.target.starts_with("urn:") {
                    link.target.clone()
                } else {
                    format!("urn:synapse:entity:{}", link.target.replace(" ", "_"))
                };

                let obj = NamedNode::new(target_uri).unwrap();

                // Añadimos el GraphName con el block_node para tener la proveniencia exacta (N-Quads)
                quads.push(Quad::new(
                    subject.clone(),
                    pred,
                    Term::NamedNode(obj),
                    oxrdf::GraphName::NamedNode(block_node.clone()),
                ));

                // Relacionar el block con el archivo original para rastreabilidad
                quads.push(Quad::new(
                    Subject::NamedNode(block_node.clone()),
                    p_derived.clone(),
                    Term::Literal(Literal::new_simple_literal(&self.file_path)),
                    oxrdf::GraphName::DefaultGraph,
                ));
            }
        }

        quads
    }
}
