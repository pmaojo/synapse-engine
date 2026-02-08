/// Entity disambiguation using string similarity and graph context
pub struct EntityDisambiguator {
    /// Similarity threshold (0.0 - 1.0)
    threshold: f64,
}

impl Default for EntityDisambiguator {
    fn default() -> Self {
        Self::new(0.8)
    }
}

impl EntityDisambiguator {
    pub fn new(threshold: f64) -> Self {
        Self { threshold }
    }

    /// Find similar URIs in the store based on label similarity
    pub fn find_similar(&self, uri: &str, candidates: &[String]) -> Vec<(String, f64)> {
        let uri_label = Self::extract_label(uri);

        let mut matches: Vec<(String, f64)> = candidates
            .iter()
            .filter_map(|c| {
                let candidate_label = Self::extract_label(c);
                let sim = Self::levenshtein_similarity(&uri_label, &candidate_label);
                if sim >= self.threshold {
                    Some((c.clone(), sim))
                } else {
                    None
                }
            })
            .collect();

        matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        matches
    }

    /// Extract the label part from a URI
    fn extract_label(uri: &str) -> String {
        // Handle common URI formats
        let uri = uri.trim_matches(|c| c == '<' || c == '>');
        let uri = uri.trim_end_matches('/');

        if let Some(idx) = uri.rfind('/') {
            uri[idx + 1..].to_string()
        } else if let Some(idx) = uri.rfind('#') {
            uri[idx + 1..].to_string()
        } else {
            uri.to_string()
        }
    }

    /// Calculate Levenshtein similarity (0.0 - 1.0)
    fn levenshtein_similarity(a: &str, b: &str) -> f64 {
        if a.is_empty() && b.is_empty() {
            return 1.0;
        }
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }

        let a_lower = a.to_lowercase();
        let b_lower = b.to_lowercase();

        let len_a = a_lower.chars().count();
        let len_b = b_lower.chars().count();
        let max_len = len_a.max(len_b);

        let distance = Self::levenshtein_distance(&a_lower, &b_lower);
        1.0 - (distance as f64 / max_len as f64)
    }

    /// Calculate Levenshtein edit distance
    fn levenshtein_distance(a: &str, b: &str) -> usize {
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();

        let m = a_chars.len();
        let n = b_chars.len();

        if m == 0 {
            return n;
        }
        if n == 0 {
            return m;
        }

        let mut prev: Vec<usize> = (0..=n).collect();
        let mut curr = vec![0; n + 1];

        for i in 1..=m {
            curr[0] = i;
            for j in 1..=n {
                let cost = if a_chars[i - 1] == b_chars[j - 1] {
                    0
                } else {
                    1
                };
                curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
            }
            std::mem::swap(&mut prev, &mut curr);
        }

        prev[n]
    }

    /// Suggest merges for similar entities
    pub fn suggest_merges(&self, uris: &[String]) -> Vec<(String, String, f64)> {
        let mut suggestions = Vec::new();

        for i in 0..uris.len() {
            for j in (i + 1)..uris.len() {
                let label_a = Self::extract_label(&uris[i]);
                let label_b = Self::extract_label(&uris[j]);
                let sim = Self::levenshtein_similarity(&label_a, &label_b);

                if sim >= self.threshold {
                    suggestions.push((uris[i].clone(), uris[j].clone(), sim));
                }
            }
        }

        suggestions.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        suggestions
    }
}
