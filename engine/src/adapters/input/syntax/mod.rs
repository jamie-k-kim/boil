pub mod extractors;
pub mod parser;

use boil_core::canon::{
    FileInfo, ProjectGraph,
    graph::{EdgeData, NodeData},
};
use crate::core::engine::EngineConfig;
use boil_core::utils::{collect_files, matches_globset, relative_path};
use crate::ports::InputModule;
use crate::ports::syntax::SyntaxProvider;
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

pub struct SyntaxModule {
    provider: Box<dyn SyntaxProvider>,
}

impl SyntaxModule {
    pub fn new(provider: Box<dyn SyntaxProvider>) -> Self {
        Self { provider }
    }
}

use crate::ports::input::IngestAction;

impl InputModule for SyntaxModule {
    fn ingest<'a>(
        &'a self,
        project_root: &Path,
        config: &EngineConfig,
    ) -> Result<IngestAction<'a>> {
        let files = collect_files(project_root, None)?;
        let mut local_file_infos = Vec::new();

        // 1. Parse and extract all file infos (Heavy I/O + Parsing)
        for file in files {
            let relative = relative_path(project_root, &file);

            if file.file_name().and_then(|n| n.to_str()) == Some("boil-profiles.toml") {
                continue;
            }

            if matches_globset(&config.ignore, &relative, &file) {
                continue;
            }

            let Ok(source) = std::fs::read_to_string(&file) else {
                continue;
            };

            println!("Parsing file: {:?}", file);

            if let Ok(info) = self.provider.parse_file(&file, &source) {
                local_file_infos.push(info);
            }
        }

        // Return the closure to mutate the graph sequentially
        Ok(Box::new(
            move |file_infos: &mut Vec<FileInfo>, graph: &mut ProjectGraph| {
                // Pre-calculate global reference counts
                let mut ref_counts = HashMap::new();
                for file in local_file_infos.iter() {
                    for reference in &file.references {
                        *ref_counts.entry(reference.name.clone()).or_insert(0) += 1;
                    }
                }

                let repo_node = *graph
                    .node_index
                    .get("repo:root")
                    .expect("Repository root should exist");

                // Build Hierarchy and collect symbols
                for file in local_file_infos.iter() {
                    let file_id = format!("file:{}", file.path.display());
                    let file_node = graph.graph.add_node(NodeData::File {
                        path: file.path.clone(),
                        language: format!("{:?}", file.language),
                        metadata: HashMap::new(),
                    });
                    graph.node_index.insert(file_id.clone(), file_node);
                    graph.reverse_index.insert(file_node, file_id);

                    graph
                        .graph
                        .add_edge(repo_node, file_node, EdgeData::Contains);

                    for symbol in &file.symbols {
                        let symbol_id = format!("symbol:{}:{}", file.path.display(), symbol.name);
                        let references = *ref_counts.get(&symbol.name).unwrap_or(&0);
                        let symbol_node = graph.graph.add_node(NodeData::Symbol {
                            name: symbol.name.clone(),
                            kind: symbol.kind.clone(),
                            exported: symbol.exported,
                            references,
                            metadata: HashMap::new(),
                        });
                        graph.node_index.insert(symbol_id.clone(), symbol_node);
                        graph.reverse_index.insert(symbol_node, symbol_id);

                        graph
                            .graph
                            .add_edge(file_node, symbol_node, EdgeData::Contains);
                    }
                }

                // Resolve Dependencies and Calls
                let mut global_symbol_map: HashMap<
                    String,
                    Vec<(
                        petgraph::stable_graph::NodeIndex,
                        petgraph::stable_graph::NodeIndex,
                    )>,
                > = HashMap::new();

                for file in local_file_infos.iter() {
                    let file_id = format!("file:{}", file.path.display());
                    if let Some(&file_node) = graph.node_index.get(&file_id) {
                        for symbol in &file.symbols {
                            let symbol_id =
                                format!("symbol:{}:{}", file.path.display(), symbol.name);
                            if let Some(&symbol_node) = graph.node_index.get(&symbol_id) {
                                global_symbol_map
                                    .entry(symbol.name.clone())
                                    .or_default()
                                    .push((file_node, symbol_node));
                            }
                        }
                    }
                }

                let file_map: HashMap<String, petgraph::stable_graph::NodeIndex> = graph
                    .node_index
                    .iter()
                    .filter(|(k, _)| k.starts_with("file:"))
                    .map(|(k, v)| {
                        let path_str = k.strip_prefix("file:").unwrap();
                        let stem = std::path::PathBuf::from(path_str)
                            .file_stem()
                            .unwrap()
                            .to_string_lossy()
                            .to_string();
                        (stem, *v)
                    })
                    .collect();

                for file in local_file_infos.iter() {
                    let file_id = format!("file:{}", file.path.display());
                    let file_node = *graph.node_index.get(&file_id).unwrap();

                    let mut imported_files = std::collections::HashSet::new();
                    for import in &file.imports {
                        let module_name = &import.module;

                        if let Some(&target_file_node) = file_map.get(module_name)
                            && target_file_node != file_node
                        {
                            graph
                                .graph
                                .add_edge(file_node, target_file_node, EdgeData::Imports);
                            imported_files.insert(target_file_node);
                        }
                    }

                    // Optimize caller lookup: Sort symbols by size (smallest first), then we can just linear scan but it's still O(R*S).
                    // Better: just do a simple linear scan but with early exit, or since we only care about functions,
                    // just build a simple interval list.
                    let mut valid_symbols: Vec<_> = file.symbols.iter().collect();
                    valid_symbols.sort_by_key(|s| s.byte_end - s.byte_start); // Smallest scopes first

                    for reference in &file.references {
                        let caller_node = valid_symbols
                            .iter()
                            .find(|s| {
                                reference.byte_offset >= s.byte_start
                                    && reference.byte_offset <= s.byte_end
                            })
                            .and_then(|s| {
                                let symbol_id =
                                    format!("symbol:{}:{}", file.path.display(), s.name);
                                graph.node_index.get(&symbol_id).copied()
                            });

                        let caller = caller_node.unwrap_or(file_node); // If not in a function, file is caller

                        if let Some(targets) = global_symbol_map.get(&reference.name) {
                            // SCOPE HEURISTIC
                            let mut resolved_targets = Vec::new();

                            // 1. Same File
                            for (f_node, s_node) in targets {
                                if *f_node == file_node {
                                    resolved_targets.push(*s_node);
                                }
                            }

                            // 2. Imported Files
                            if resolved_targets.is_empty() {
                                for (f_node, s_node) in targets {
                                    if imported_files.contains(f_node) {
                                        resolved_targets.push(*s_node);
                                    }
                                }
                            }

                            // 3. Global Fallback
                            if resolved_targets.is_empty() {
                                for (_, s_node) in targets {
                                    resolved_targets.push(*s_node);
                                }
                            }

                            let edge_data = match reference.kind {
                                boil_core::canon::ReferenceKind::Call => EdgeData::Calls,
                                _ => EdgeData::References,
                            };

                            for target in resolved_targets {
                                if caller != target {
                                    graph.graph.add_edge(caller, target, edge_data.clone());
                                }
                            }
                        }
                    }
                }

                for file in local_file_infos.iter_mut() {
                    for symbol in file.symbols.iter_mut() {
                        if let Some(count) = ref_counts.get(&symbol.name) {
                            symbol.references = *count;
                        }
                    }
                }

                file_infos.extend(local_file_infos);

                Ok(())
            },
        ))
    }
}
