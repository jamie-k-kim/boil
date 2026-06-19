//! Dynamic runtime trace adapters.
//!
//! These adapters ingest execution traces (e.g., from OpenTelemetry, JSON logs)
//! to map dynamic runtime behaviour (call graphs, latencies) onto the static canon graph.

pub mod json;
pub mod opentelemetry;

pub use json::JsonTraceProvider;
pub use opentelemetry::OpentelemetryProvider;
