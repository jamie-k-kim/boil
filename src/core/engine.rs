use crate::core::canon::{FileInfo, ProjectGraph};
use crate::ports::{InputModule, OutputModule, ReasoningModule};
use anyhow::Result;
use std::path::Path;

/// Configuration settings for the boil core engine execution.
pub struct EngineConfig {
    /// Compiled glob sets specifying directories/files to ignore.
    pub ignore: globset::GlobSet,
    /// Raw string glob patterns to ignore during scanning.
    pub ignore_patterns: Vec<String>,
    /// Optional timestamp to override the default metadata timestamps in the graph.
    pub force_timestamp: Option<String>,
    /// Whether to suppress standard log/info console outputs.
    pub silent: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            ignore: globset::GlobSet::empty(),
            ignore_patterns: Vec::new(),
            force_timestamp: None,
            silent: true,
        }
    }
}

/// The central coordinator/orchestrator of the boil platform.
///
/// The `Engine` registers input modules (to ingest codebase information),
/// reasoning modules (to infer connections and structure), and output modules
/// (to serialize or export the final canon). It runs them in a coordinated pipeline.
pub struct Engine {
    inputs: Vec<Box<dyn InputModule>>,
    reasoning: Vec<Box<dyn ReasoningModule>>,
    outputs: Vec<Box<dyn OutputModule>>,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    /// Creates a new, empty engine instance with no modules registered.
    pub fn new() -> Self {
        Self {
            inputs: Vec::new(),
            reasoning: Vec::new(),
            outputs: Vec::new(),
        }
    }

    /// Registers an input module to ingest data.
    pub fn register_input(mut self, module: Box<dyn InputModule>) -> Self {
        self.inputs.push(module);
        self
    }

    /// Registers a reasoning module to process the ingested graph.
    pub fn register_reasoning(mut self, module: Box<dyn ReasoningModule>) -> Self {
        self.reasoning.push(module);
        self
    }

    /// Registers an output module to export the finalized canon.
    pub fn register_output(mut self, module: Box<dyn OutputModule>) -> Self {
        self.outputs.push(module);
        self
    }

    /// Runs the engine pipeline in headless mode (ingestion and reasoning only).
    ///
    /// This returns the parsed file information list and the constructed project graph.
    pub fn run_headless(
        &self,
        project_root: &Path,
        config: &EngineConfig,
    ) -> Result<(Vec<FileInfo>, ProjectGraph)> {
        use rayon::prelude::*;

        let mut file_infos = Vec::new();
        let mut graph = ProjectGraph::empty();

        // Pre-initialize Repository Root
        let repo_node = graph
            .graph
            .add_node(crate::core::canon::graph::NodeData::Repository {
                path: std::path::PathBuf::from("."),
                metadata: std::collections::HashMap::new(),
            });
        graph.node_index.insert("repo:root".to_string(), repo_node);
        graph
            .reverse_index
            .insert(repo_node, "repo:root".to_string());

        // 1. Concurrent Analysis (Ingest)
        let actions: Result<Vec<_>> = self
            .inputs
            .par_iter()
            .map(|input| input.ingest(project_root, config))
            .collect();

        // Apply mutations sequentially to maintain topological determinism
        for action in actions? {
            action(&mut file_infos, &mut graph)?;
        }

        // 2. Reason
        for reasoning in &self.reasoning {
            reasoning.process(config, &mut file_infos, &mut graph)?;
        }

        Ok((file_infos, graph))
    }

    /// Runs the full engine pipeline (ingestion, reasoning, and output exports).
    ///
    /// Returns the finalized file information list and project graph.
    pub fn run(
        &self,
        project_root: &Path,
        output_root: &Path,
        config: &EngineConfig,
    ) -> Result<(Vec<FileInfo>, ProjectGraph)> {
        let (mut file_infos, mut graph) = self.run_headless(project_root, config)?;

        // 3. Output
        for output in &self.outputs {
            output.export(
                project_root,
                output_root,
                config,
                &mut file_infos,
                &mut graph,
            )?;
        }

        Ok((file_infos, graph))
    }
}
