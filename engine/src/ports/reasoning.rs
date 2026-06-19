use boil_core::canon::{FileInfo, ProjectGraph};
use crate::core::engine::EngineConfig;
use anyhow::Result;

/// A port defining reasoning modules that post-process the canonical graph.
///
/// Reasoning modules run after all input modules have ingested data. They analyze the
/// constructed graph to extract higher-level intelligence (e.g., semantic similarity,
/// Leiden clustering, architecture layout).
pub trait ReasoningModule {
    /// Evaluates the existing file infos and graph, appending or mutating them with inferred metadata.
    fn process(
        &self,
        config: &EngineConfig,
        file_infos: &mut Vec<FileInfo>,
        graph: &mut ProjectGraph,
    ) -> Result<()>;
}
