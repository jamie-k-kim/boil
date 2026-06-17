use anyhow::Result;

/// A trait for generating vector embeddings from text strings.
///
/// Embedding providers can wrap local models (such as ONNX-based FastEmbed)
/// or call external APIs (such as OpenAI, Cohere, Ollama, VoyageAI).
pub trait EmbeddingProvider: Send + Sync {
    /// Generates vector embeddings for a batch of input texts.
    ///
    /// Returns a list of floating-point vector embeddings in the same order
    /// as the input texts.
    fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>>;
}
