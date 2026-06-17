use crate::core::language::Language;
use std::path::PathBuf;

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Interface,
    Trait,
    Module,
    Constant,
    Variable,
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub byte_start: usize,
    pub byte_end: usize,
    pub line_start: usize,
    pub line_end: usize,
    pub exported: bool,
    pub signature: Option<String>,
    pub references: usize, // Tracks how many times this specific symbol is used project-wide
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Import {
    pub module: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ReferenceKind {
    Call,
    Type,
    Variable,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Reference {
    pub name: String,       // Name of the symbol being referenced
    pub byte_offset: usize, // Position in the source file
    pub kind: ReferenceKind,
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileInfo {
    pub path: PathBuf,
    pub language: Language,
    pub symbols: Vec<Symbol>,
    pub imports: Vec<Import>,
    pub references: Vec<Reference>,
    pub original_tokens: usize, // Tracks original tokens for budget calculations
}
