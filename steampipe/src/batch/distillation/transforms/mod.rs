pub use strategy::{CompressionStrategy, get_cached_query, has_overlap};

pub mod comments_ast;
pub mod skeleton;
pub mod literals;
pub mod debug_remover;
pub mod symbol_pruner;
pub mod import_simplifier;
pub mod strategy;