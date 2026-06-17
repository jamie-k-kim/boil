use crate::core::canon::{
    FileInfo, ProjectGraph,
    graph::{EdgeData, NodeData},
};
use crate::core::engine::EngineConfig;
use crate::core::utils::relative_path;
use crate::ports::InputModule;
use crate::ports::ownership::OwnershipProvider;
use anyhow::Result;
use globset::Glob;
use std::collections::HashMap;
use std::path::Path;

pub struct OwnershipModule {
    provider: Box<dyn OwnershipProvider>,
}

impl OwnershipModule {
    pub fn new(provider: Box<dyn OwnershipProvider>) -> Self {
        Self { provider }
    }
}

use crate::ports::input::IngestAction;

impl InputModule for OwnershipModule {
    fn ingest<'a>(
        &'a self,
        project_root: &Path,
        _config: &EngineConfig,
    ) -> Result<IngestAction<'a>> {
        let rules = self.provider.get_ownership_info(project_root)?;

        let mut compiled_rules = Vec::new();
        for rule in rules {
            let mut pattern = rule.pattern.clone();
            if pattern.starts_with('/') {
                pattern = pattern[1..].to_string();
            }
            if pattern.ends_with('/') {
                pattern.push_str("**");
            }

            if let Ok(glob) = Glob::new(&pattern) {
                compiled_rules.push((glob.compile_matcher(), rule.owners));
            }
        }

        let project_root_buf = project_root.to_path_buf();
        Ok(Box::new(
            move |_file_infos: &mut Vec<FileInfo>, graph: &mut ProjectGraph| {
                let mut file_indices = Vec::new();
                for node_idx in graph.graph.node_indices() {
                    if let NodeData::File { path, .. } = &graph.graph[node_idx] {
                        file_indices.push((node_idx, path.clone()));
                    }
                }

                for (file_node, path) in file_indices {
                    let rel_path = relative_path(&project_root_buf, &path);

                    let mut final_owners = None;
                    for (matcher, owners) in &compiled_rules {
                        if matcher.is_match(&rel_path) {
                            final_owners = Some(owners);
                        }
                    }

                    if let Some(owners) = final_owners {
                        for owner_name in owners {
                            let owner_id = format!("owner:{}", owner_name);
                            let owner_node =
                                *graph.node_index.entry(owner_id.clone()).or_insert_with(|| {
                                    graph.graph.add_node(NodeData::Owner {
                                        name: owner_name.clone(),
                                        metadata: HashMap::new(),
                                    })
                                });
                            graph.reverse_index.insert(owner_node, owner_id);

                            graph
                                .graph
                                .add_edge(file_node, owner_node, EdgeData::OwnedBy);
                        }
                    }
                }
                Ok(())
            },
        ))
    }
}
