use crate::ports::embeddings::EmbeddingProvider;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct OllamaEmbeddingRequest {
    model: String,
    prompt: String,
}

#[derive(Deserialize)]
struct OllamaEmbeddingResponse {
    embedding: Vec<f32>,
}

pub struct OllamaProvider {
    base_url: String,
    model: String,
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl OllamaProvider {
    pub fn new() -> Self {
        let base_url = std::env::var("OLLAMA_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());

        Self {
            base_url,
            model: "nomic-embed-text".to_string(), // Default model
        }
    }
}

impl EmbeddingProvider for OllamaProvider {
    fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let mut embeddings = Vec::new();
        let endpoint = format!("{}/api/embeddings", self.base_url);

        // Ollama `/api/embeddings` typically takes a single prompt
        for text in texts {
            let request_body = OllamaEmbeddingRequest {
                model: self.model.clone(),
                prompt: text,
            };

            let response: OllamaEmbeddingResponse = ureq::post(&endpoint)
                .send_json(&request_body)?
                .body_mut()
                .read_json()?;

            embeddings.push(response.embedding);
        }

        Ok(embeddings)
    }
}
