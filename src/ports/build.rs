use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Represents a package dependency declared in build configuration files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildDependency {
    /// The name of the dependency package.
    pub name: String,
    /// The specified version constraint or exact version.
    pub version: String,
    /// Whether the dependency is external (e.g. from a package registry)
    /// or internal (e.g. a workspace path dependency).
    pub is_external: bool,
}

/// Metadata summarizing a workspace package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    /// The package name.
    pub name: String,
    /// The package version.
    pub version: String,
    /// The absolute or relative path to the package directory.
    pub path: PathBuf,
    /// The list of direct dependencies of this package.
    pub dependencies: Vec<BuildDependency>,
}

/// A port defining operations for analyzing build workspaces and extracting package structures.
///
/// Implementations (e.g., Cargo, Gradle, Bazel) parse build definition files to mapping out
/// target/dependency relationships.
pub trait BuildProvider: Send + Sync {
    /// Analyzes the build files under the project root and returns a list of packages and dependencies.
    fn analyze_workspace(&self, project_root: &Path) -> Result<Vec<PackageInfo>>;
}
