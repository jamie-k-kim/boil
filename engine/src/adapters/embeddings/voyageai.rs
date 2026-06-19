use crate::ports::embeddings::EmbeddingProvider;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct VoyageAiEmbeddingRequest {
    input: Vec<String>,
    model: String,
}

#[derive(Deserialize)]
struct VoyageAiEmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

pub struct VoyageAiProvider {
    api_key: String,
    model: String,
}

impl Default for VoyageAiProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl VoyageAiProvider {
    pub fn new() -> Self {
        let api_key = std::env::var("VOYAGE_API_KEY")
            .expect("VOYAGE_API_KEY environment variable is required for Voyage AI embeddings");

        Self {
            api_key,
            model: "voyage-code-2".to_string(), // Default model
        }
    }
}

impl EmbeddingProvider for VoyageAiProvider {
    fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let request_body = VoyageAiEmbeddingRequest {
            input: texts,
            model: self.model.clone(),
        };

        let response: VoyageAiEmbeddingResponse =
            ureq::post("https://api.voyageai.com/v1/embeddings")
                .header("Authorization", &format!("Bearer {}", self.api_key))
                .send_json(&request_body)?
                .body_mut()
                .read_json()?;

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
