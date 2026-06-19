use crate::ports::clustering::{ClusteringProvider, ClusteringResult};
use anyhow::Result;
use leiden_rs::{GraphDataBuilder, Leiden, LeidenConfig};
use std::collections::HashMap;

/// A clustering provider that uses the Leiden algorithm for community detection.
///
/// The Leiden algorithm is a state-of-the-art method for detecting communities in large networks,
/// improving upon the Louvain method by guaranteeing well-connected communities.
pub struct LeidenClusteringProvider;

impl ClusteringProvider for LeidenClusteringProvider {
    fn cluster(
        &self,
        node_count: usize,
        edges: Vec<(usize, usize, f64)>,
    ) -> Result<ClusteringResult> {
        let mut builder = GraphDataBuilder::new(node_count);
        for (u, v, w) in edges {
            builder
                .add_edge(u, v, w)
                .map_err(|e| anyhow::anyhow!("Failed to add edge: {:?}", e))?;
        }
        let graph_data = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build graph data: {:?}", e))?;

        let config = LeidenConfig::default();
        let result = Leiden::new(config)
            .run(&graph_data)
            .map_err(|e| anyhow::anyhow!("Leiden algorithm failed: {:?}", e))?;

        let mut communities = HashMap::new();
        for (community_id, node_indices) in result.partition.communities() {
            communities.insert(community_id, node_indices.to_vec());
        }

        Ok(ClusteringResult { communities })
    }
}
