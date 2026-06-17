//! # Ports (Hexagonal Architecture)
//!
//! The `ports` module defines the interfaces (traits) that external adapters must implement
//! to interact with the boil engine. Following the ports-and-adapters architecture, these
//! traits abstract away the implementation details of various data sources (e.g., Git,
//! Tree-sitter, PagerDuty, Neo4j) so the core engine can consume them uniformly.

pub mod build;
pub mod clustering;
pub mod documentation;
pub mod embeddings;
pub mod export;
pub mod input;
pub mod output;
pub mod ownership;
pub mod reasoning;
pub mod runtime;
pub mod syntax;
pub mod temporal;
pub mod vcs;

pub use build::{BuildDependency, BuildProvider, PackageInfo};
pub use clustering::{ClusteringProvider, ClusteringResult};
pub use documentation::DocumentationProvider;
pub use embeddings::EmbeddingProvider;
pub use export::ExportProvider;
pub use input::InputModule;
pub use output::OutputModule;
pub use ownership::OwnershipProvider;
pub use reasoning::ReasoningModule;
pub use runtime::{RuntimeProvider, RuntimeTrace};
pub use syntax::SyntaxProvider;
pub use temporal::{DiffReport, TemporalProvider};
pub use vcs::{Author, VcsProvider};
