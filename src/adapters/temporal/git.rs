use crate::core::canon::ProjectGraph;
use crate::ports::temporal::{DiffReport, TemporalProvider};
use anyhow::Result;
use git2::Repository;
use std::path::Path;

/// A temporal provider driven by Git.
///
/// Efficiently checks out Git trees from specific commit hashes or branches into
/// memory or temporary workspaces to perform multi-revision graph diffing.
pub struct GitTemporalProvider;

impl Default for GitTemporalProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl GitTemporalProvider {
    pub fn new() -> Self {
        Self
    }
}

impl TemporalProvider for GitTemporalProvider {
    fn build_graph_from_commit(
        &self,
        engine: &crate::core::Engine,
        config: &crate::core::EngineConfig,
        repo_path: &Path,
        commit_rev: &str,
    ) -> Result<ProjectGraph> {
        let temp_dir = tempfile::tempdir()?;
        let temp_path = temp_dir.path();

        let repo = Repository::open(repo_path)?;
        let object = repo.revparse_single(commit_rev)?;
        let commit = object
            .as_commit()
            .ok_or_else(|| anyhow::anyhow!("Object is not a commit"))?;
        let tree = commit.tree()?;

        tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
            if entry.kind() == Some(git2::ObjectType::Blob) {
                let file_path = temp_path.join(root).join(entry.name().unwrap_or(""));
                if let Some(parent) = file_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if let Ok(blob) = repo.find_blob(entry.id()) {
                    let _ = std::fs::write(&file_path, blob.content());
                }
            }
            git2::TreeWalkResult::Ok
        })?;

        let (_, graph) = engine.run_headless(temp_path, config)?;

        Ok(graph)
    }

    fn compare_graphs(&self, base: &ProjectGraph, head: &ProjectGraph) -> Result<DiffReport> {
        let mut report = DiffReport {
            added_subsystems: Vec::new(),
            removed_subsystems: Vec::new(),
            added_symbols: Vec::new(),
            removed_symbols: Vec::new(),
            moved_symbols: Vec::new(),
            new_edges: 0,
        };

        let base_subsystems: std::collections::HashSet<String> = base
            .node_index
            .keys()
            .filter(|k| k.starts_with("subsystem:"))
            .cloned()
            .collect();
        let head_subsystems: std::collections::HashSet<String> = head
            .node_index
            .keys()
            .filter(|k| k.starts_with("subsystem:"))
            .cloned()
            .collect();

        for sub in head_subsystems.difference(&base_subsystems) {
            report.added_subsystems.push(sub.clone());
        }
        for sub in base_subsystems.difference(&head_subsystems) {
            report.removed_subsystems.push(sub.clone());
        }

        let mut base_entity_map = std::collections::HashMap::new();
        for id in base.node_index.keys() {
            if id.starts_with("symbol:") {
                let parts: Vec<&str> = id.split(':').collect();
                if parts.len() >= 3 {
                    let path = parts[1];
                    let name = parts[2];
                    base_entity_map.insert(name.to_string(), path.to_string());
                }
            }
        }

        let mut head_entity_map = std::collections::HashMap::new();
        for id in head.node_index.keys() {
            if id.starts_with("symbol:") {
                let parts: Vec<&str> = id.split(':').collect();
                if parts.len() >= 3 {
                    let path = parts[1];
                    let name = parts[2];
                    head_entity_map.insert(name.to_string(), path.to_string());
                }
            }
        }

        for (name, head_path) in &head_entity_map {
            match base_entity_map.get(name) {
                Some(base_path) => {
                    if base_path != head_path {
                        report
                            .moved_symbols
                            .push((name.clone(), format!("{} -> {}", base_path, head_path)));
                    }
                }
                None => report.added_symbols.push(name.clone()),
            }
        }

        for name in base_entity_map.keys() {
            if !head_entity_map.contains_key(name) {
                report.removed_symbols.push(name.clone());
            }
        }

        let base_edges = base.graph.edge_count();
        let head_edges = head.graph.edge_count();
        if head_edges > base_edges {
            report.new_edges = head_edges - base_edges;
        }

        Ok(report)
    }
}
