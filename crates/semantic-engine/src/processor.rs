/// Simple semantic chunker for text processing
pub struct TextProcessor;

impl Default for TextProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl TextProcessor {
    pub fn new() -> Self {
        Self
    }

    /// Split text into recursive chunks
    /// Simple implementation: split by double newline (paragraphs), then by newline, then by length
    pub fn chunk_text(&self, text: &str, max_chars: usize) -> Vec<String> {
        let mut chunks = Vec::new();

        // 1. Split by paragraphs
        let paragraphs: Vec<&str> = text.split("\n\n").collect();

        for p in paragraphs {
            if p.len() <= max_chars {
                if !p.trim().is_empty() {
                    chunks.push(p.to_string());
                }
            } else {
                // 2. Split by sentences (naive period check) or newlines
                let lines: Vec<&str> = p.split('\n').collect();
                let mut current_chunk = String::new();

                for line in lines {
                    if current_chunk.len() + line.len() + 1 > max_chars {
                        if !current_chunk.is_empty() {
                            chunks.push(current_chunk.clone());
                            current_chunk.clear();
                        }
                        // If line itself is too long, we truncate/force split (simplification)
                        if line.len() > max_chars {
                            chunks.push(line[..max_chars].to_string());
                            // Drop remainder for simplicity in this MVP
                        } else {
                            current_chunk = line.to_string();
                        }
                    } else {
                        if !current_chunk.is_empty() {
                            current_chunk.push('\n');
                        }
                        current_chunk.push_str(line);
                    }
                }
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk);
                }
            }
        }

        chunks
    }
}
