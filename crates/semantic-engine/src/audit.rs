use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

/// Record of an inference operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRecord {
    pub timestamp: DateTime<Utc>,
    pub namespace: String,
    pub strategy: String,
    pub input_triples: usize,
    pub inferred_triples: usize,
    pub duplicates_skipped: usize,
    pub sample_inferences: Vec<(String, String, String)>,
}

/// Audit trail for tracking inference operations
pub struct InferenceAudit {
    /// Namespace -> inference records
    records: RwLock<HashMap<String, Vec<InferenceRecord>>>,
    /// Maximum records per namespace
    max_records: usize,
}

impl Default for InferenceAudit {
    fn default() -> Self {
        Self::new()
    }
}

impl InferenceAudit {
    pub fn new() -> Self {
        Self {
            records: RwLock::new(HashMap::new()),
            max_records: 100,
        }
    }

    /// Log an inference operation
    pub fn log(
        &self,
        namespace: &str,
        strategy: &str,
        input: usize,
        inferred: usize,
        skipped: usize,
        samples: Vec<(String, String, String)>,
    ) {
        let record = InferenceRecord {
            timestamp: Utc::now(),
            namespace: namespace.to_string(),
            strategy: strategy.to_string(),
            input_triples: input,
            inferred_triples: inferred,
            duplicates_skipped: skipped,
            sample_inferences: samples.into_iter().take(10).collect(),
        };

        let mut records = self.records.write().unwrap();
        let ns_records = records.entry(namespace.to_string()).or_default();

        ns_records.push(record);

        // Trim to max records
        if ns_records.len() > self.max_records {
            ns_records.remove(0);
        }
    }

    /// Get inference history for a namespace
    pub fn get_history(&self, namespace: &str) -> Vec<InferenceRecord> {
        let records = self.records.read().unwrap();
        records.get(namespace).cloned().unwrap_or_default()
    }

    /// Get last inference for a namespace
    pub fn get_last(&self, namespace: &str) -> Option<InferenceRecord> {
        let records = self.records.read().unwrap();
        records.get(namespace).and_then(|r| r.last().cloned())
    }

    /// Export all records as JSON
    pub fn export_json(&self) -> String {
        let records = self.records.read().unwrap();
        serde_json::to_string_pretty(&*records).unwrap_or_default()
    }
}
