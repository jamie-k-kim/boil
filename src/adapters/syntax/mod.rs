//! AST parsing adapters.
//!
//! These adapters parse raw source code files into Abstract Syntax Trees (ASTs)
//! and extract fundamental canon nodes such as Classes, Functions, and Imports.

pub mod treesitter;
pub use treesitter::TreeSitterProvider;
