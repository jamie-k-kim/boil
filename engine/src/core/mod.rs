//! # Core Engine
//!
//! The `core` module contains the central domain logic and the "Engine" of the boil platform.
//! It defines the canonical graph schema (`canon`) and coordinates the ingestion of data from
//! various plugins (adapters) to construct the architectural knowledge graph. This module is
//! independent of any specific external tools or data sources.

pub mod canon;
pub mod engine;

pub use engine::{Engine, EngineConfig};
