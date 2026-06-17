use crate::core::canon::{FileInfo, ProjectGraph, graph::EdgeData};
use crate::core::engine::EngineConfig;
use crate::ports::InputModule;
use crate::ports::runtime::RuntimeProvider;
use anyhow::Result;
use std::path::Path;

pub struct RuntimeModule {
    provider: Box<dyn RuntimeProvider>,
}

impl RuntimeModule {
    pub fn new(provider: Box<dyn RuntimeProvider>) -> Self {
        Self { provider }
    }
}

use crate::ports::input::IngestAction;

impl InputModule for RuntimeModule {
    fn ingest<'a>(
        &'a self,
        project_root: &Path,
        _config: &EngineConfig,
    ) -> Result<IngestAction<'a>> {
        let traces = self.provider.get_traces(project_root)?;

        let project_root_buf = project_root.to_path_buf();
        Ok(Box::new(
            move |_file_infos: &mut Vec<FileInfo>, graph: &mut ProjectGraph| {
                let repo_node = *graph
                    .node_index
                    .get("repo:root")
                    .expect("Repository root should exist");

                for entry in traces {
                    let symbol_id = format!(
                        "symbol:{}:{}",
                        project_root_buf.join(&entry.file).display(),
                        entry.symbol
                    );
                    if let Some(&symbol_node) = graph.node_index.get(&symbol_id) {
                        graph.graph[symbol_node].get_metadata_mut().insert(
                            "runtime_executions".to_string(),
                            entry.count.to_string(),
                        );
                        graph
                            .graph
                            .add_edge(repo_node, symbol_node, EdgeData::ExecutedAtRuntime);
                    }
                }
                Ok(())
            },
        ))
    }
}
