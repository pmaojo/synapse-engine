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

    /// Split text into recursive chunks with overlap
    pub fn chunk_text(&self, text: &str, max_chars: usize, overlap: usize) -> Vec<String> {
        let mut chunks = Vec::new();
        // Simple approach: Split by whitespace to preserve words
        let words: Vec<&str> = text.split_inclusive(char::is_whitespace).collect();

        let mut current_chunk = String::new();
        let mut current_len = 0;
        let mut current_words: Vec<&str> = Vec::new();

        for word in words {
            if current_len + word.len() > max_chars {
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk.trim().to_string());
                }

                // Handle overlap
                let mut overlap_words = Vec::new();
                let mut overlap_len = 0;

                // Backtrack to capture overlap context
                for w in current_words.iter().rev() {
                    if overlap_len + w.len() <= overlap {
                        overlap_words.push(*w);
                        overlap_len += w.len();
                    } else {
                        break;
                    }
                }
                overlap_words.reverse();

                current_chunk = overlap_words.concat();
                current_len = overlap_len;
                current_words = overlap_words;
            }

            current_chunk.push_str(word);
            current_len += word.len();
            current_words.push(word);
        }

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
    fn test_chunk_text_with_overlap() {
        let processor = TextProcessor::new();
        let text = "one two three four five six seven eight nine ten";
        // max_chars small to force split. "one two " is 8 chars.
        // Let's use max_chars=15.
        // "one two three" = 13 chars. " four" = 5. Total 18 > 15.
        // So chunk 1: "one two three" (13)
        // Overlap: say 10 chars.
        // "three" is 5 chars. "two " is 4. "one " is 4.
        // overlap 10 captures "two three".
        // next chunk start with "two three".
        // "two three four five" = 19 chars > 15.
        // So chunk 2: "two three four" (14).

        // Wait, my implementation uses words.
        // Let's test with overlap parameter.

        let chunks = processor.chunk_text(text, 15, 6); // overlap 6 chars

        // chunk 1: "one two three" (13 chars).
        // overlap logic:
        // current_words: ["one", " ", "two", " ", "three"]
        // overlap=6.
        // "three" (5) <= 6. Keep. len=5.
        // " " (1) <= 6-5=1. Keep. len=6.
        // "two" (3) > 0. Stop.
        // overlap words: [" ", "three"] -> " three"
        // next starts with " three".

        // loop continues. next word " ". " three " (7).
        // "four". " three four" (11).
        // " ". " three four " (12).
        // "five". " three four five" (16) > 15.
        // chunk 2: "three four" (trimmed) -> "three four" (10 chars).

        // verify
        println!("Chunks: {:?}", chunks);
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0], "one two three");
        assert!(chunks[1].contains("three")); // overlap worked
    }
}
