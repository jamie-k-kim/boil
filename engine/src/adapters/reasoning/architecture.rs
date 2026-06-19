use boil_core::canon::{
    FileInfo, ProjectGraph,
    graph::{EdgeData, NodeData},
};
use crate::core::engine::EngineConfig;
use crate::ports::ReasoningModule;
use crate::ports::clustering::ClusteringProvider;
use anyhow::Result;
use petgraph::stable_graph::NodeIndex;
use petgraph::visit::EdgeRef;
use std::collections::HashMap;

pub struct ArchitectureAnalyzer {
    clustering: Box<dyn ClusteringProvider>,
}

impl ArchitectureAnalyzer {
    pub fn new(clustering: Box<dyn ClusteringProvider>) -> Self {
        Self { clustering }
    }
}

impl ReasoningModule for ArchitectureAnalyzer {
    fn process(
        &self,
        _config: &EngineConfig,
        _file_infos: &mut Vec<FileInfo>,
        graph: &mut ProjectGraph,
    ) -> Result<()> {
        let mut file_indices = Vec::new();
        for node_idx in graph.graph.node_indices() {
            if let NodeData::File { .. } = &graph.graph[node_idx] {
                file_indices.push(node_idx);
            }
        }

        if file_indices.is_empty() {
            return Ok(());
        }

        let node_to_leiden: HashMap<NodeIndex, usize> = file_indices
            .iter()
            .enumerate()
            .map(|(i, &idx)| (idx, i))
            .collect();
        let leiden_to_node: Vec<NodeIndex> = file_indices.clone();
        let mut edges_map: HashMap<(usize, usize), f64> = HashMap::new();

        for edge_idx in graph.graph.edge_indices() {
            let (u, v) = graph.graph.edge_endpoints(edge_idx).unwrap();
            let u_file = find_containing_file(graph, u);
            let v_file = find_containing_file(graph, v);

            if let (Some(u_f), Some(v_f)) = (u_file, v_file) {
                if u_f == v_f {
                    continue;
                }

                let u_idx = *node_to_leiden.get(&u_f).unwrap();
                let v_idx = *node_to_leiden.get(&v_f).unwrap();
                let pair = if u_idx < v_idx {
                    (u_idx, v_idx)
                } else {
                    (v_idx, u_idx)
                };

                let weight = match &graph.graph[edge_idx] {
                    EdgeData::Calls => 1.0,
                    EdgeData::Imports => 0.5,
                    _ => 0.0,
                };

                *edges_map.entry(pair).or_insert(0.0) += weight;
            }
        }

        let edges_vec: Vec<(usize, usize, f64)> =
            edges_map.iter().map(|(&(u, v), &w)| (u, v, w)).collect();
        let result = self.clustering.cluster(file_indices.len(), edges_vec)?;

        let repo_node = *graph.node_index.get("repo:root").unwrap();

        for (community_id, node_indices) in result.communities {
            let mut file_paths = Vec::new();
            let mut file_in_degrees: HashMap<NodeIndex, f64> = HashMap::new();
            let cluster_set: std::collections::HashSet<usize> =
                node_indices.iter().copied().collect();

            for &leiden_idx in &node_indices {
                let file_node = leiden_to_node[leiden_idx];
                file_in_degrees.insert(file_node, 0.0);
                if let NodeData::File { path, .. } = &graph.graph[file_node] {
                    file_paths.push(path.clone());
                }
            }

            for (&(u_idx, v_idx), &weight) in &edges_map {
                if cluster_set.contains(&u_idx) && cluster_set.contains(&v_idx) {
                    *file_in_degrees.get_mut(&leiden_to_node[u_idx]).unwrap() += weight;
                    *file_in_degrees.get_mut(&leiden_to_node[v_idx]).unwrap() += weight;
                }
            }

            let mut hub_file = None;
            let mut max_degree = -1.0;
            for (&file_node, &degree) in &file_in_degrees {
                if degree > max_degree {
                    max_degree = degree;
                    if let NodeData::File { path, .. } = &graph.graph[file_node] {
                        hub_file = Some(
                            path.file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string(),
                        );
                    }
                }
            }
            let hub_str = hub_file.unwrap_or_else(|| "unknown".to_string());

            let mut dir_counts: HashMap<String, usize> = HashMap::new();
            for path in &file_paths {
                if let Some(parent) = path.parent() {
                    let dir_str = parent.to_string_lossy().to_string();
                    if !dir_str.is_empty() && dir_str != "." {
                        *dir_counts.entry(dir_str).or_insert(0) += 1;
                    }
                }
            }

            let mut primary_dir = String::from("root");
            let mut max_count = 0;
            for (dir, &count) in &dir_counts {
                if count > max_count {
                    max_count = count;
                    primary_dir = dir.clone();
                }
            }

            let subsystem_name = format!(
                "{} (Hub: {})",
                primary_dir.replace("/", "_").replace("\\", "_"),
                hub_str
            );

            let mut metadata = HashMap::new();
            metadata.insert("file_count".to_string(), file_paths.len().to_string());
            metadata.insert("primary_dir".to_string(), primary_dir);
            metadata.insert("hub_file".to_string(), hub_str);

            let subsystem_node = graph.graph.add_node(NodeData::Subsystem {
                name: subsystem_name.clone(),
                metadata,
            });
            let subsystem_id = format!("subsystem:{}", community_id);
            graph
                .node_index
                .insert(subsystem_id.clone(), subsystem_node);
            graph.reverse_index.insert(subsystem_node, subsystem_id);

            graph
                .graph
                .add_edge(repo_node, subsystem_node, EdgeData::Contains);

            for leiden_idx in &node_indices {
                let file_node = leiden_to_node[*leiden_idx];

                if let Some(old_edge) = graph.graph.find_edge(repo_node, file_node) {
                    graph.graph.remove_edge(old_edge);
                }

                graph
                    .graph
                    .add_edge(subsystem_node, file_node, EdgeData::Contains);
            }
        }

        Ok(())
    }
}

fn find_containing_file(graph: &ProjectGraph, node_idx: NodeIndex) -> Option<NodeIndex> {
    match &graph.graph[node_idx] {
        NodeData::File { .. } => Some(node_idx),
        NodeData::Symbol { .. } => {
            for edge in graph.graph.edges_directed(node_idx, petgraph::Incoming) {
                if let EdgeData::Contains = &graph.graph[edge.id()] {
                    let parent = edge.source();
                    if let NodeData::File { .. } = &graph.graph[parent] {
                        return Some(parent);
                    }
                }
            }
            None
        }
        _ => None,
    }
}
