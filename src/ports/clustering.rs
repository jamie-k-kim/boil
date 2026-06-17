use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Identifies a community/subsystem (cluster) within the graph.
pub type CommunityId = usize;
/// Identifies a node by its petgraph index inside the stable graph.
pub type NodeId = usize;

/// The results of a subsystem clustering process.
#[derive(Serialize, Deserialize)]
pub struct ClusteringResult {
    /// Maps each community identifier to the list of node indices belonging to it.
    pub communities: HashMap<CommunityId, Vec<NodeId>>,
}

/// A port defining operations for partitioning/clustering a project graph into communities.
///
/// Implementations (e.g. Leiden algorithm) find cohesive subdivisions of the architecture.
pub trait ClusteringProvider: Send + Sync {
    /// Performs clustering/community detection on the graph nodes and edges.
    fn cluster(
        &self,
        node_count: usize,
        edges: Vec<(NodeId, NodeId, f64)>,
    ) -> Result<ClusteringResult>;
}
