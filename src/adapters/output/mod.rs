//! Output modules.
//!
//! Output modules execute at the end of the Boil pipeline. They take the fully enriched
//! canon graph and write it to various external destinations (e.g., JSON, Graph databases).

pub mod knowledge_export;

pub use knowledge_export::KnowledgeExportModule;
