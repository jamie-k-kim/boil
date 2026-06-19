//! Input modules.
//!
//! Input modules execute at the start of the Boil pipeline. They gather raw data
//! (e.g., AST syntax, git history, dependencies) and populate the initial canon graph.

pub mod build;
pub mod documentation;
pub mod ownership;
pub mod provenance;
pub mod runtime;
pub mod syntax;

pub use build::BuildModule;
pub use documentation::DocumentationModule;
pub use ownership::OwnershipModule;
pub use provenance::ProvenanceModule;
pub use runtime::RuntimeModule;
pub use syntax::SyntaxModule;
