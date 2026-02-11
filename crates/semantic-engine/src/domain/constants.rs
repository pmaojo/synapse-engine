/// Weight decay for scores when expanding the graph during hybrid search.
pub const GRAPH_EXPANSION_DECAY: f32 = 0.8;

/// Base score for a node found at the direct starting point.
pub const BASE_PATH_SCORE: f32 = 1.0;

/// Default character length for text chunks during ingestion.
pub const DEFAULT_CHUNK_SIZE: usize = 1000;

/// Default overlap for text chunks.
pub const DEFAULT_CHUNK_OVERLAP: usize = 150;

/// Ontology Prefixes
pub const SYS_ONTOLOGY_BASE: &str = "http://sys.semantic/core#";
pub const FRONTEND_ONTOLOGY_BASE: &str = "http://sys.semantic/frontend#";
pub const PROV_WAS_DERIVED_FROM: &str = "http://www.w3.org/ns/prov#wasDerivedFrom";
pub const PROV_GENERATED_AT_TIME: &str = "http://www.w3.org/ns/prov#generatedAtTime";
pub const PROV_WAS_GENERATED_BY: &str = "http://www.w3.org/ns/prov#wasGeneratedBy";
pub const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
