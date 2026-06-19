use crate::ports::embeddings::EmbeddingProvider;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct OpenAiEmbeddingRequest {
    input: Vec<String>,
    model: String,
}

#[derive(Deserialize)]
struct OpenAiEmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

pub struct OpenAiProvider {
    api_key: String,
    model: String,
}

impl Default for OpenAiProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenAiProvider {
    pub fn new() -> Self {
        let api_key = std::env::var("OPENAI_API_KEY")
            .expect("OPENAI_API_KEY environment variable is required for OpenAI embeddings");

        Self {
            api_key,
            model: "text-embedding-3-small".to_string(), // Default model
        }
    }
}

impl EmbeddingProvider for OpenAiProvider {
    fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let request_body = OpenAiEmbeddingRequest {
            input: texts,
            model: self.model.clone(),
        };

        let response: OpenAiEmbeddingResponse = ureq::post("https://api.openai.com/v1/embeddings")
            .header("Authorization", &format!("Bearer {}", self.api_key))
            .send_json(&request_body)?
            .body_mut()
            .read_json()?;

        // OpenAI returns data array, we need to sort it by index just in case and map to Vec<Vec<f32>>
        let mut embeddings_with_index: Vec<(usize, Vec<f32>)> = response
            .data
            .into_iter()
            .map(|data| (data.index, data.embedding))
            .collect();

        embeddings_with_index.sort_by_key(|(index, _)| *index);

        let embeddings: Vec<Vec<f32>> = embeddings_with_index
            .into_iter()
            .map(|(_, embedding)| embedding)
            .collect();

        Ok(embeddings)
    }
}
