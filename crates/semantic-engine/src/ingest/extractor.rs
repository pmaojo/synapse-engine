pub struct ExtractedTriple {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}

pub fn extract_metadata(content: &str, source_path: &str) -> Vec<ExtractedTriple> {
    let mut triples = Vec::new();
    let mut current_header = String::new();
    let _filename = std::path::Path::new(source_path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(header) = trimmed.strip_prefix('#') {
            current_header = header.trim_start_matches('#').trim().to_string();
            // Link file to header
            triples.push(ExtractedTriple {
                subject: format!("file://{}", source_path),
                predicate: "http://synapse.os/contains_section".to_string(),
                object: current_header.clone(),
            });
        } else if let Some(item) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            if !current_header.is_empty() {
                triples.push(ExtractedTriple {
                    subject: current_header.clone(),
                    predicate: "http://synapse.os/has_list_item".to_string(),
                    object: item.trim().to_string(),
                });
            }
        } else if trimmed.contains(':') {
            let parts: Vec<&str> = trimmed.splitn(2, ':').collect();
            if parts.len() == 2 {
                let key = parts[0].trim();
                let value = parts[1].trim();
                if !key.is_empty() && !value.is_empty() {
                    let subject = if current_header.is_empty() {
                        format!("file://{}", source_path)
                    } else {
                        current_header.clone()
                    };

                    triples.push(ExtractedTriple {
                        subject,
                        predicate: format!("http://synapse.os/property/{}", key.replace(" ", "_")),
                        object: value.to_string(),
                    });
                }
            }
        }
    }

    triples
}
