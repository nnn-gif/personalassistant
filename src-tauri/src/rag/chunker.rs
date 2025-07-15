use crate::error::Result;

pub struct TextChunker {
    chunk_size: usize,
    overlap: usize,
}

impl TextChunker {
    pub fn new() -> Self {
        Self {
            chunk_size: 1000,
            overlap: 200,
        }
    }

    pub fn new_with_config(chunk_size: usize, overlap: usize) -> Self {
        Self {
            chunk_size,
            overlap,
        }
    }

    pub fn chunk_text(&self, text: &str) -> Result<Vec<String>> {
        if text.is_empty() {
            return Ok(vec![]);
        }

        let chunks = self.chunk_text_by_size(text);

        if chunks.is_empty() {
            // If no chunks were created, return the original text as a single chunk
            Ok(vec![text.to_string()])
        } else {
            Ok(chunks)
        }
    }

    fn chunk_text_by_size(&self, text: &str) -> Vec<String> {
        let mut chunks = Vec::new();
        let chars: Vec<char> = text.chars().collect();
        let total_len = chars.len();

        if total_len <= self.chunk_size {
            return vec![text.to_string()];
        }

        let mut start = 0;

        while start < total_len {
            let end = std::cmp::min(start + self.chunk_size, total_len);
            let chunk: String = chars[start..end].iter().collect();
            chunks.push(chunk);

            if end >= total_len {
                break;
            }

            // Move start position with overlap
            start = end.saturating_sub(self.overlap);
        }

        chunks
    }

    pub fn chunk_text_by_sentences(&self, text: &str) -> Result<Vec<String>> {
        if text.is_empty() {
            return Ok(vec![]);
        }

        let sentences = self.split_into_sentences(text);
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let target_chunk_size = 1000;

        for sentence in sentences {
            if current_chunk.len() + sentence.len() > target_chunk_size && !current_chunk.is_empty()
            {
                chunks.push(current_chunk.trim().to_string());
                current_chunk = sentence;
            } else {
                if !current_chunk.is_empty() {
                    current_chunk.push(' ');
                }
                current_chunk.push_str(&sentence);
            }
        }

        if !current_chunk.is_empty() {
            chunks.push(current_chunk.trim().to_string());
        }

        Ok(chunks)
    }

    pub fn chunk_text_by_paragraphs(&self, text: &str) -> Result<Vec<String>> {
        if text.is_empty() {
            return Ok(vec![]);
        }

        let paragraphs: Vec<&str> = text.split("\n\n").collect();
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let target_chunk_size = 1000;

        for paragraph in paragraphs {
            let paragraph = paragraph.trim();
            if paragraph.is_empty() {
                continue;
            }

            if current_chunk.len() + paragraph.len() > target_chunk_size
                && !current_chunk.is_empty()
            {
                chunks.push(current_chunk.trim().to_string());
                current_chunk = paragraph.to_string();
            } else {
                if !current_chunk.is_empty() {
                    current_chunk.push_str("\n\n");
                }
                current_chunk.push_str(paragraph);
            }
        }

        if !current_chunk.is_empty() {
            chunks.push(current_chunk.trim().to_string());
        }

        Ok(chunks)
    }

    fn split_into_sentences(&self, text: &str) -> Vec<String> {
        // Simple sentence splitting - in production you might want to use a more sophisticated approach
        let sentence_endings = ['.', '!', '?'];
        let mut sentences = Vec::new();
        let mut current_sentence = String::new();

        for char in text.chars() {
            current_sentence.push(char);

            if sentence_endings.contains(&char) {
                let sentence = current_sentence.trim().to_string();
                if !sentence.is_empty() {
                    sentences.push(sentence);
                }
                current_sentence.clear();
            }
        }

        // Add any remaining text as a sentence
        if !current_sentence.trim().is_empty() {
            sentences.push(current_sentence.trim().to_string());
        }

        sentences
    }

    pub fn get_chunk_metadata(
        &self,
        chunk: &str,
        chunk_index: usize,
    ) -> std::collections::HashMap<String, String> {
        let mut metadata = std::collections::HashMap::new();

        metadata.insert("chunk_index".to_string(), chunk_index.to_string());
        metadata.insert("chunk_length".to_string(), chunk.len().to_string());
        metadata.insert(
            "word_count".to_string(),
            chunk.split_whitespace().count().to_string(),
        );

        // Add some basic content analysis
        let line_count = chunk.lines().count();
        metadata.insert("line_count".to_string(), line_count.to_string());

        // Check for code patterns
        if chunk.contains("fn ")
            || chunk.contains("def ")
            || chunk.contains("function ")
            || chunk.contains("class ")
        {
            metadata.insert("content_type".to_string(), "code".to_string());
        } else if chunk.contains("# ") || chunk.contains("## ") || chunk.contains("### ") {
            metadata.insert("content_type".to_string(), "markdown".to_string());
        } else {
            metadata.insert("content_type".to_string(), "text".to_string());
        }

        metadata
    }
}
