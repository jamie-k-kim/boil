use boil_core::canon::FileInfo;
use anyhow::Result;
use std::path::Path;

/// A port defining syntax analysis operations.
///
/// Implementations (e.g. tree-sitter providers) parse file sources into abstract syntax
/// structures like symbols, imports, and cross-references.
pub trait SyntaxProvider: Send + Sync {
    /// Parses a file's contents into structured `FileInfo` symbols and imports.
    fn parse_file(&self, path: &Path, source: &str) -> Result<FileInfo>;
}
