use anyhow::Result;

pub struct ExtractionResult {
    pub triples: Vec<(String, String, String)>,
}

pub trait Extractor {
    fn extract(&self, content: &str) -> Result<ExtractionResult>;
}

pub struct CsvExtractor {
    pub delimiter: u8,
}

impl Default for CsvExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl CsvExtractor {
    pub fn new() -> Self {
        Self { delimiter: b',' }
    }
}

impl Extractor for CsvExtractor {
    fn extract(&self, content: &str) -> Result<ExtractionResult> {
        let mut rdr = csv::ReaderBuilder::new()
            .delimiter(self.delimiter)
            .from_reader(content.as_bytes());

        let headers = rdr.headers()?.clone();
        let mut triples = Vec::new();

        for result in rdr.records() {
            let record = result?;
            if let Some(subject) = record.get(0) {
                if subject.trim().is_empty() {
                    continue;
                }

                for (i, value) in record.iter().enumerate().skip(1) {
                    if let Some(predicate) = headers.get(i) {
                        let val_trimmed = value.trim();
                        if !val_trimmed.is_empty() {
                            triples.push((
                                subject.to_string(),
                                predicate.to_string(),
                                val_trimmed.to_string(),
                            ));
                        }
                    }
                }
            }
        }

        Ok(ExtractionResult { triples })
    }
}

pub struct MarkdownExtractor;

impl Extractor for MarkdownExtractor {
    fn extract(&self, content: &str) -> Result<ExtractionResult> {
        let mut triples = Vec::new();
        let mut current_header = String::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if trimmed.starts_with("#") {
                current_header = trimmed.trim_start_matches('#').trim().to_string();
            } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                if !current_header.is_empty() {
                    let item = trimmed[2..].trim();
                    if !item.is_empty() {
                        triples.push((
                            current_header.clone(),
                            "mentions".to_string(),
                            item.to_string(),
                        ));
                    }
                }
            } else if trimmed.contains(":") {
                let parts: Vec<&str> = trimmed.splitn(2, ':').collect();
                if parts.len() == 2 && !current_header.is_empty() {
                    let predicate = parts[0].trim();
                    let object = parts[1].trim();
                    triples.push((
                        current_header.clone(),
                        predicate.to_string(),
                        object.to_string(),
                    ));
                }
            }
        }

        Ok(ExtractionResult { triples })
    }
}
