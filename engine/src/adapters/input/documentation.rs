use boil_core::canon::{FileInfo, ProjectGraph};
use crate::core::engine::EngineConfig;
use crate::ports::InputModule;
use crate::ports::documentation::DocumentationProvider;
use anyhow::Result;
use std::path::Path;

pub struct DocumentationModule {
    provider: Box<dyn DocumentationProvider>,
}

impl DocumentationModule {
    pub fn new(provider: Box<dyn DocumentationProvider>) -> Self {
        Self { provider }
    }
}

use crate::ports::input::IngestAction;

impl InputModule for DocumentationModule {
    fn ingest<'a>(
        &'a self,
        project_root: &Path,
        _config: &EngineConfig,
    ) -> Result<IngestAction<'a>> {
        let documents = self.provider.analyze_workspace(project_root)?;

        Ok(Box::new(
            move |_file_infos: &mut Vec<FileInfo>, graph: &mut ProjectGraph| {
                let repo_node = *graph
                    .node_index
                    .get("repo:root")
                    .expect("Repository root should exist");

                for doc in documents {
                    let doc_id = format!("document:{}", doc.path.display());
                    let doc_node =
                        graph
                            .graph
                            .add_node(boil_core::canon::graph::NodeData::Document {
                                path: doc.path.clone(),
                                kind: doc.kind.clone(),
                                metadata: doc.sections.clone(),
                            });

                    graph.node_index.insert(doc_id.clone(), doc_node);
                    graph.reverse_index.insert(doc_node, doc_id);
                    graph.graph.add_edge(
                        repo_node,
                        doc_node,
                        boil_core::canon::graph::EdgeData::Contains,
                    );
                    graph.graph.add_edge(
                        doc_node,
                        repo_node,
                        boil_core::canon::graph::EdgeData::Describes,
                    );
                }
                Ok(())
            },
        ))
    }
}
