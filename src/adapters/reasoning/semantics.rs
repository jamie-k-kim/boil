use crate::core::canon::{
    FileInfo, ProjectGraph,
    graph::{EdgeData, NodeData},
};
use crate::core::engine::EngineConfig;
use crate::ports::ReasoningModule;
use crate::ports::embeddings::EmbeddingProvider;
use anyhow::Result;

pub struct SemanticsModule {
    provider: Box<dyn EmbeddingProvider>,
}

impl SemanticsModule {
    pub fn new(provider: Box<dyn EmbeddingProvider>) -> Self {
        Self { provider }
    }
}

fn normalize_identifier(ident: &str) -> String {
    let mut result = String::new();
    let mut prev_is_lower = false;
    for c in ident.chars() {
        if c == '_' {
            result.push(' ');
            prev_is_lower = false;
        } else if c.is_uppercase() {
            if prev_is_lower {
                result.push(' ');
            }
            result.push(c);
            prev_is_lower = false;
        } else {
            result.push(c);
            prev_is_lower = true;
        }
    }
    result.to_lowercase()
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;
    for (va, vb) in a.iter().zip(b.iter()) {
        dot += (*va * *vb) as f64;
        norm_a += (*va * *va) as f64;
        norm_b += (*vb * *vb) as f64;
    }
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a.sqrt() * norm_b.sqrt())
}

impl ReasoningModule for SemanticsModule {
    fn process(
        &self,
        _config: &EngineConfig,
        _file_infos: &mut Vec<FileInfo>,
        graph: &mut ProjectGraph,
    ) -> Result<()> {
        let mut node_texts = Vec::new();
        let mut node_indices = Vec::new();

        for node_idx in graph.graph.node_indices() {
            if let NodeData::Symbol { name, kind, .. } = &graph.graph[node_idx] {
                let normalized = normalize_identifier(name);
                let text = format!("{:?} {}", kind, normalized);
                node_texts.push(text);
                node_indices.push(node_idx);
            }
        }

        if node_texts.is_empty() {
            return Ok(());
        }

        if let Ok(embeddings) = self.provider.embed(node_texts) {
            let threshold = 0.85;

            for i in 0..embeddings.len() {
                for j in (i + 1)..embeddings.len() {
                    let sim = cosine_similarity(&embeddings[i], &embeddings[j]);
                    if sim > threshold {
                        let u = node_indices[i];
                        let v = node_indices[j];
                        graph
                            .graph
                            .add_edge(u, v, EdgeData::SemanticSimilarity(sim));
                    }
                }

                let node_idx = node_indices[i];
                let mut node_data = graph.graph[node_idx].clone();
                let emb_str = serde_json::to_string(&embeddings[i]).unwrap();
                node_data
                    .get_metadata_mut()
                    .insert("embedding".to_string(), emb_str);
                graph.graph[node_idx] = node_data;
            }
        }
        Ok(())
    }
}
