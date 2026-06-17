use crate::core::canon::{
    FileInfo, ProjectGraph,
    graph::{EdgeData, NodeData},
};
use crate::core::engine::EngineConfig;
use crate::ports::InputModule;
use crate::ports::vcs::FileProvenance;
use crate::ports::vcs::VcsProvider;
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

pub struct ProvenanceModule {
    vcs: Box<dyn VcsProvider>,
}

impl ProvenanceModule {
    pub fn new(vcs: Box<dyn VcsProvider>) -> Self {
        Self { vcs }
    }
}

use crate::ports::input::IngestAction;

impl InputModule for ProvenanceModule {
    fn ingest<'a>(
        &'a self,
        project_root: &Path,
        _config: &EngineConfig,
    ) -> Result<IngestAction<'a>> {
        // Run get_file_provenance concurrently for all files we care about (or let the apply phase do it lazily).
        // Since we want to offload I/O, we can pre-fetch file provenance.
        let files = crate::core::utils::collect_files(project_root, None).unwrap_or_default();
        use rayon::prelude::*;

        let file_provs: HashMap<std::path::PathBuf, FileProvenance> = files
            .par_iter()
            .filter_map(|f| {
                self.vcs
                    .get_file_provenance(project_root, f)
                    .ok()
                    .map(|prov| (f.clone(), prov))
            })
            .collect();

        // Pre-fetch symbol authors by just warming up the blame cache for all files concurrently
        files.par_iter().for_each(|f| {
            let _ = self.vcs.get_author_at_line(project_root, f, 0);
        });

        let project_root_buf = project_root.to_path_buf();
        // Apply Phase
        Ok(Box::new(
            move |file_infos: &mut Vec<FileInfo>, graph: &mut ProjectGraph| {
                for file in file_infos.iter() {
                    let file_id = format!("file:{}", file.path.display());

                    // Add File Provenance
                    if let Some(file_prov) = file_provs.get(&file.path).cloned().or_else(|| {
                        self.vcs
                            .get_file_provenance(&project_root_buf, &file.path)
                            .ok()
                    }) && let Some(&file_node) = graph.node_index.get(&file_id)
                    {
                        if let Some(NodeData::File { metadata, .. }) =
                            graph.graph.node_weight_mut(file_node)
                        {
                            metadata
                                .insert("commits".to_string(), file_prov.commit_count.to_string());
                            metadata.insert("created".to_string(), file_prov.creation_date.clone());
                            metadata.insert(
                                "modified".to_string(),
                                file_prov.last_modified_date.clone(),
                            );
                        }

                        for author in &file_prov.primary_authors {
                            let author_id = format!("author:{}", author.email);
                            let author_node = *graph
                                .node_index
                                .entry(author_id.clone())
                                .or_insert_with(|| {
                                    graph.graph.add_node(NodeData::Author {
                                        name: author.name.clone(),
                                        email: author.email.clone(),
                                        metadata: HashMap::new(),
                                    })
                                });
                            graph.reverse_index.insert(author_node, author_id);
                            graph
                                .graph
                                .add_edge(file_node, author_node, EdgeData::AuthoredBy);
                        }
                    }

                    // Add Symbol Provenance
                    for symbol in &file.symbols {
                        if let Ok(author) = self.vcs.get_author_at_line(
                            &project_root_buf,
                            &file.path,
                            symbol.line_start,
                        ) {
                            let author_id = format!("author:{}", author.email);
                            let author_node = *graph
                                .node_index
                                .entry(author_id.clone())
                                .or_insert_with(|| {
                                    graph.graph.add_node(NodeData::Author {
                                        name: author.name.clone(),
                                        email: author.email.clone(),
                                        metadata: HashMap::new(),
                                    })
                                });
                            graph.reverse_index.insert(author_node, author_id);

                            let symbol_id =
                                format!("symbol:{}:{}", file.path.display(), symbol.name);
                            if let Some(&symbol_node) = graph.node_index.get(&symbol_id) {
                                graph.graph.add_edge(
                                    symbol_node,
                                    author_node,
                                    EdgeData::AuthoredBy,
                                );
                            }
                        }
                    }
                }
                Ok(())
            },
        ))
    }
}
