use crate::ports::embeddings::EmbeddingProvider;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct CohereEmbeddingRequest {
    texts: Vec<String>,
    model: String,
    input_type: String,
}

#[derive(Deserialize)]
struct CohereEmbeddingResponse {
    embeddings: Vec<Vec<f32>>,
}

pub struct CohereProvider {
    api_key: String,
    model: String,
}

impl Default for CohereProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CohereProvider {
    pub fn new() -> Self {
        let api_key = std::env::var("COHERE_API_KEY")
            .expect("COHERE_API_KEY environment variable is required for Cohere embeddings");

        Self {
            api_key,
            model: "embed-english-v3.0".to_string(), // Default model
        }
    }
}

impl EmbeddingProvider for CohereProvider {
    fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let request_body = CohereEmbeddingRequest {
            texts,
            model: self.model.clone(),
            input_type: "search_document".to_string(), // For codebase/architectural embedding
        };

        let response: CohereEmbeddingResponse = ureq::post("https://api.cohere.com/v1/embed")
            .header("Authorization", &format!("Bearer {}", self.api_key))
            .send_json(&request_body)?
            .body_mut()
            .read_json()?;

        Ok(response.embeddings)
    }
}
