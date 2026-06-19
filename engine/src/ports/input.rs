use boil_core::canon::{FileInfo, ProjectGraph};
use crate::core::engine::EngineConfig;
use anyhow::Result;
use std::path::Path;

/// A deferred mutation/action returned by an input module's concurrent analysis.
///
/// This action is executed sequentially on the main thread to apply the results of
/// the module's analysis to the shared `FileInfo` vector and `ProjectGraph` canon.
pub type IngestAction<'a> =
    Box<dyn FnOnce(&mut Vec<FileInfo>, &mut ProjectGraph) -> Result<()> + Send + 'a>;

/// A port defining input modules that extract information from the codebase.
///
/// Implementations (e.g., syntax parser, build parsing, git blame) ingest raw project files
/// and return a transaction-like callback that updates the knowledge graph.
pub trait InputModule: Send + Sync {
    /// Ingests codebase metadata concurrently and returns a callback to mutate the canonical graph.
    fn ingest<'a>(&'a self, project_root: &Path, config: &EngineConfig)
    -> Result<IngestAction<'a>>;
}
