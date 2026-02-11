use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorerOptions {
    pub namespace: String,
    pub node_id: u32,
    pub depth: usize,
    pub direction: ExplorerDirection,
    pub scoring_strategy: ScoringStrategy,
    pub edge_filter: Option<String>,
    pub node_type_filter: Option<String>,
    pub limit_per_layer: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExplorerDirection {
    Outgoing,
    Incoming,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScoringStrategy {
    Path,
    Degree,
}

impl ExplorerDirection {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "incoming" => Self::Incoming,
            "both" => Self::Both,
            _ => Self::Outgoing,
        }
    }

    pub fn is_outgoing(&self) -> bool {
        matches!(self, Self::Outgoing | Self::Both)
    }

    pub fn is_incoming(&self) -> bool {
        matches!(self, Self::Incoming | Self::Both)
    }
}

impl ScoringStrategy {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "degree" => Self::Degree,
            _ => Self::Path,
        }
    }
}
