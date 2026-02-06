use anyhow::Result;
use html2text::from_read;
use std::io::Cursor;

/// Configuration for text processing and chunking.
#[derive(Debug, Clone)]
pub struct ProcessorConfig {
    /// Maximum number of characters per chunk.
    pub chunk_size: usize,
    /// Number of characters to overlap between chunks.
    pub chunk_overlap: usize,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            chunk_size: 1000,
            chunk_overlap: 200,
        }
    }
}

/// Advanced processor for text and HTML content.
pub struct Processor {
    config: ProcessorConfig,
}

impl Processor {
    /// Creates a new Processor with the given configuration.
    pub fn new(config: ProcessorConfig) -> Self {
        Self { config }
    }

    /// Processes HTML content: sanitizes it to text and then chunks it.
    pub fn process_html(&self, html: &str) -> Result<Vec<String>> {
        // Use a reasonable width for text wrapping, e.g., 120.
        // This helps maintain some structure while converting to text.
        let text = from_read(Cursor::new(html), 120).map_err(|e| anyhow::anyhow!(e))?;
        Ok(self.chunk_text(&text))
    }

    /// Splits text into overlapping chunks based on the configuration.
    /// Tries to split on whitespace to preserve word boundaries.
    pub fn chunk_text(&self, text: &str) -> Vec<String> {
        if text.is_empty() {
            return Vec::new();
        }

        let mut chunks = Vec::new();
        // Split by whitespace but keep the delimiter to reconstruct faithfully
        let words: Vec<&str> = text.split_inclusive(char::is_whitespace).collect();

        let mut current_chunk = String::new();
        let mut current_len = 0;
        // Keep track of words in the current chunk to handle overlap efficiently
        let mut current_words: Vec<&str> = Vec::new();

        for word in words {
            let word_len = word.len();

            // If adding this word exceeds chunk_size, we finalize the current chunk
            if current_len + word_len > self.config.chunk_size {
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk.trim().to_string());
                }

                // Prepare the next chunk with overlap
                let mut overlap_chunk = String::new();
                let mut overlap_len = 0;
                let mut new_current_words = Vec::new();

                // Work backwards to find how many words fit in the overlap
                for w in current_words.iter().rev() {
                    if overlap_len + w.len() <= self.config.chunk_overlap {
                        new_current_words.push(*w);
                        overlap_len += w.len();
                    } else {
                        break;
                    }
                }
                new_current_words.reverse();

                // Reconstruct the overlap string
                for w in &new_current_words {
                    overlap_chunk.push_str(w);
                }

                current_chunk = overlap_chunk;
                current_len = overlap_len;
                current_words = new_current_words;
            }

            current_chunk.push_str(word);
            current_len += word_len;
            current_words.push(word);
        }

        // Add the last chunk if not empty
        if !current_chunk.is_empty() {
            chunks.push(current_chunk.trim().to_string());
        }

        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_text_simple() {
        let config = ProcessorConfig {
            chunk_size: 10,
            chunk_overlap: 0,
        };
        let processor = Processor::new(config);
        let text = "one two three four";
        let chunks = processor.chunk_text(text);

        // "one two " is 8 chars. "one two three" is 13 > 10.
        // So chunk 1: "one two"
        // chunk 2: "three four"

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0], "one two");
        assert_eq!(chunks[1], "three four");
    }

    #[test]
    fn test_chunk_text_overlap() {
        let config = ProcessorConfig {
            chunk_size: 15,
            chunk_overlap: 6,
        };
        let processor = Processor::new(config);
        let text = "one two three four five";
        // "one two three" = 13 chars. " four" = 5. Total 18 > 15.
        // Chunk 1: "one two three"
        // Overlap: 6 chars. "three" (5) + " " (1) = 6.
        // Next chunk starts with " three".
        // " three four" = 11. " five" = 5. Total 16 > 15.
        // Wait, " three four" is 11. " five" is 5. 11+5 = 16.
        // So chunk 2: "three four"
        // Overlap: 6 chars. "four" (4) + " " (1) = 5. "three" (5). 5 < 6.
        // " three" (6).
        // Next chunk starts with " four".
        // " four five" = 10.

        let chunks = processor.chunk_text(text);

        assert!(chunks.len() >= 2);
        assert_eq!(chunks[0], "one two three");
        assert!(chunks[1].contains("three"));
    }

    #[test]
    fn test_process_html() {
        let config = ProcessorConfig::default();
        let processor = Processor::new(config);
        let html = "<html><body><h1>Title</h1><p>Paragraph 1.</p></body></html>";

        let chunks = processor.process_html(html).unwrap();
        assert!(!chunks.is_empty());
        // html2text should convert h1 to # Title or similar depending on width, or just Title
        // With width 120, it likely preserves some formatting or just outputs text.
        // html2text default behavior for h1 is typically underlined or capitalized.

        // Just check that we got some text back and tags are gone
        let combined = chunks.join(" ");
        assert!(combined.contains("Title"));
        assert!(combined.contains("Paragraph 1"));
        assert!(!combined.contains("<html>"));
    }
}
