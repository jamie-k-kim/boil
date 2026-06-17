//! Vector embedding providers.
//!
//! These adapters convert source code nodes and documentation chunks into dense
//! vector embeddings for semantic search and graph clustering algorithms.

pub mod cohere;
pub mod fastembed;
pub mod ollama;
pub mod openai;
pub mod voyageai;

pub use cohere::CohereProvider;
pub use fastembed::FastEmbedProvider;
pub use ollama::OllamaProvider;
pub use voyageai::VoyageAiProvider;
