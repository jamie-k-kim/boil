use boil_core::canon::{
    FileInfo, ProjectGraph,
    graph::{EdgeData, NodeData},
};
use crate::core::engine::EngineConfig;
use crate::ports::InputModule;
use crate::ports::build::BuildProvider;
use anyhow::Result;
use petgraph::visit::EdgeRef;
use std::collections::HashMap;
use std::path::Path;

pub struct BuildModule {
    provider: Box<dyn BuildProvider>,
}

impl BuildModule {
    pub fn new(provider: Box<dyn BuildProvider>) -> Self {
        Self { provider }
    }
}

use crate::ports::input::IngestAction;

impl InputModule for BuildModule {
    fn ingest<'a>(
        &'a self,
        project_root: &Path,
        _config: &EngineConfig,
    ) -> Result<IngestAction<'a>> {
        let packages = self.provider.analyze_workspace(project_root)?;

        let project_root_buf = project_root.to_path_buf();
        Ok(Box::new(
            move |_file_infos: &mut Vec<FileInfo>, graph: &mut ProjectGraph| {
                let repo_node = *graph
                    .node_index
                    .get("repo:root")
                    .expect("Repository root should exist");

                // Deduplicate external packages by name
                let mut external_nodes: HashMap<String, petgraph::stable_graph::NodeIndex> =
                    HashMap::new();

                for pkg in packages {
                    let pkg_id = format!("package:{}", pkg.name);
                    let pkg_node = *graph.node_index.entry(pkg_id.clone()).or_insert_with(|| {
                        let node = graph.graph.add_node(NodeData::Package {
                            name: pkg.name.clone(),
                            version: pkg.version.clone(),
                            metadata: HashMap::new(),
                        });
                        graph.graph.add_edge(repo_node, node, EdgeData::Contains);
                        node
                    });
                    graph.reverse_index.insert(pkg_node, pkg_id.clone());

                    // Link files to this package if they reside in its directory
                    let file_nodes_to_link: Vec<petgraph::stable_graph::NodeIndex> = graph
                        .node_index
                        .iter()
                        .filter(|(k, _)| k.starts_with("file:"))
                        .filter(|(k, _)| {
                            // Check if file path starts with package path
                            // e.g. file:testing/tmp-input/src/main.rs vs pkg_dir: testing/tmp-input
                            let file_path = k.strip_prefix("file:").unwrap();
                            let abs_file = project_root_buf.join(file_path);
                            abs_file.starts_with(&pkg.path)
                        })
                        .map(|(_, &v)| v)
                        .collect();

                    for file_node in file_nodes_to_link {
                        // Check if edge already exists to prevent duplicate Contains
                        let mut exists = false;
                        for edge in graph.graph.edges_directed(pkg_node, petgraph::Outgoing) {
                            if edge.target() == file_node
                                && let EdgeData::Contains = edge.weight()
                            {
                                exists = true;
                                break;
                            }
                        }
                        if !exists {
                            graph
                                .graph
                                .add_edge(pkg_node, file_node, EdgeData::Contains);
                        }
                    }

                    // Add dependencies
                    for dep in pkg.dependencies {
                        let dep_id = format!("package:{}", dep.name);

                        // If it's another internal package, it will be handled (or already exists) in node_index
                        // Otherwise, put it in external_nodes
                        let dep_node = if let Some(&node) = graph.node_index.get(&dep_id) {
                            node
                        } else {
                            *external_nodes.entry(dep.name.clone()).or_insert_with(|| {
                                let node = graph.graph.add_node(NodeData::Package {
                                    name: dep.name.clone(),
                                    version: dep.version.clone(),
                                    metadata: HashMap::new(),
                                });
                                graph.node_index.insert(dep_id.clone(), node);
                                node
                            })
                        };
                        graph.reverse_index.insert(dep_node, dep_id);

                        graph
                            .graph
                            .add_edge(pkg_node, dep_node, EdgeData::DependsOnExternal);
                    }
                }

                Ok(())
            },
        ))
    }
}
