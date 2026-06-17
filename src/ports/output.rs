use crate::core::canon::{FileInfo, ProjectGraph};
use crate::core::engine::EngineConfig;
use anyhow::Result;
use std::path::Path;

/// A port defining output modules that consume the completed graph.
///
/// Output modules run after the core ingestion and reasoning phases are complete.
/// They are responsible for exporting or displaying the structural canonical state.
pub trait OutputModule {
    /// Exports or saves the finalized project canon graph and files.
    fn export(
        &self,
        project_root: &Path,
        output_root: &Path,
        config: &EngineConfig,
        file_infos: &mut Vec<FileInfo>,
        graph: &mut ProjectGraph,
    ) -> Result<()>;
}
