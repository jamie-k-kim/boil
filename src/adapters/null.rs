use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

use crate::core::canon::FileInfo;
use crate::core::canon::ProjectGraph;
use crate::ports::build::{BuildProvider, PackageInfo};
use crate::ports::clustering::{ClusteringProvider, ClusteringResult, NodeId};
use crate::ports::documentation::DocumentationProvider;
use crate::ports::embeddings::EmbeddingProvider;
use crate::ports::ownership::{OwnershipProvider, OwnershipRule};
use crate::ports::runtime::{RuntimeProvider, RuntimeTrace};
use crate::ports::syntax::SyntaxProvider;
use crate::ports::temporal::{DiffReport, TemporalProvider};
use crate::ports::vcs::{Author, FileProvenance, VcsProvider};
pub struct NullVcsProvider;
impl VcsProvider for NullVcsProvider {
    fn get_file_provenance(
        &self,
        _project_root: &Path,
        _file_path: &Path,
    ) -> Result<FileProvenance> {
        Ok(FileProvenance {
            primary_authors: vec![],
            commit_count: 0,
            creation_date: String::new(),
            last_modified_date: String::new(),
        })
    }
    fn get_author_at_line(
        &self,
        _project_root: &Path,
        _file_path: &Path,
        _line: usize,
    ) -> Result<Author> {
        Ok(Author {
            name: "Unknown".to_string(),
            email: "unknown@example.com".to_string(),
        })
    }
}

pub struct NullSyntaxProvider;
impl SyntaxProvider for NullSyntaxProvider {
    fn parse_file(&self, path: &Path, _source: &str) -> Result<FileInfo> {
        Ok(FileInfo {
            path: path.to_path_buf(),
            language: crate::core::language::Language::Unknown,
            symbols: vec![],
            imports: vec![],
            references: vec![],
            original_tokens: 0,
        })
    }
}

pub struct NullBuildProvider;
impl BuildProvider for NullBuildProvider {
    fn analyze_workspace(&self, _project_root: &Path) -> Result<Vec<PackageInfo>> {
        Ok(vec![])
    }
}

pub struct NullRuntimeProvider;
impl RuntimeProvider for NullRuntimeProvider {
    fn get_traces(&self, _project_root: &Path) -> Result<Vec<RuntimeTrace>> {
        Ok(vec![])
    }
}

pub struct NullOwnershipProvider;
impl OwnershipProvider for NullOwnershipProvider {
    fn get_ownership_info(&self, _project_root: &Path) -> Result<Vec<OwnershipRule>> {
        Ok(vec![])
    }
}

pub struct NullDocumentationProvider;
impl DocumentationProvider for NullDocumentationProvider {
    fn analyze_workspace(
        &self,
        _project_root: &Path,
    ) -> Result<Vec<crate::ports::documentation::DocumentInfo>> {
        Ok(vec![])
    }
}

pub struct NullClusteringProvider;
impl ClusteringProvider for NullClusteringProvider {
    fn cluster(
        &self,
        _node_count: usize,
        _edges: Vec<(NodeId, NodeId, f64)>,
    ) -> Result<ClusteringResult> {
        Ok(ClusteringResult {
            communities: HashMap::new(),
        })
    }
}

pub struct NullEmbeddingProvider;
impl EmbeddingProvider for NullEmbeddingProvider {
    fn embed(&self, _texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        Ok(vec![])
    }
}

pub struct NullTemporalProvider;
impl TemporalProvider for NullTemporalProvider {
    fn build_graph_from_commit(
        &self,
        _engine: &crate::core::Engine,
        _config: &crate::core::EngineConfig,
        _repo_path: &Path,
        _commit_rev: &str,
    ) -> Result<ProjectGraph> {
        Ok(ProjectGraph::empty())
    }
    fn compare_graphs(&self, _base: &ProjectGraph, _head: &ProjectGraph) -> Result<DiffReport> {
        Ok(DiffReport {
            added_subsystems: vec![],
            removed_subsystems: vec![],
            added_symbols: vec![],
            removed_symbols: vec![],
            moved_symbols: vec![],
            new_edges: 0,
        })
    }
}
