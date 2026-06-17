//! Architecture clustering adapters.
//!
//! These adapters analyze the topology of the canon graph and partition nodes
//! into functional communities or logical sub-systems using graph algorithms.

pub mod leiden;
pub use leiden::LeidenClusteringProvider;
