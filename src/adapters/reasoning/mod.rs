//! Reasoning modules.
//!
//! Reasoning modules execute in the middle of the pipeline. They analyze the initial
//! graph to discover derived knowledge (e.g., architectural clusters, semantic embeddings)
//! and enrich the graph iteratively.

pub mod architecture;
pub mod semantics;

pub use architecture::ArchitectureAnalyzer;
pub use semantics::SemanticsModule;
