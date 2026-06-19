use crate::canon::SymbolKind;
use petgraph::Directed;
use petgraph::stable_graph::{NodeIndex, StableGraph};
use petgraph::visit::EdgeRef;
use std::collections::HashMap;
use std::path::PathBuf;

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum NodeData {
    Repository {
        path: PathBuf,
        metadata: HashMap<String, String>,
    },
    Subsystem {
        name: String,
        metadata: HashMap<String, String>,
    },
    Module {
        name: String,
        metadata: HashMap<String, String>,
    },
    File {
        path: PathBuf,
        language: String,
        metadata: HashMap<String, String>,
    },
    Symbol {
        name: String,
        kind: SymbolKind,
        exported: bool,
        references: usize,
        metadata: HashMap<String, String>,
    },
    Package {
        name: String,
        version: String,
        metadata: HashMap<String, String>,
    },
    BuildTarget {
        name: String,
        kind: String,
        metadata: HashMap<String, String>,
    },
    Author {
        name: String,
        email: String,
        metadata: HashMap<String, String>,
    },
    Owner {
        name: String,
        metadata: HashMap<String, String>,
    },
    Document {
        path: PathBuf,
        kind: String,
        metadata: HashMap<String, String>,
    },
}

impl NodeData {
    pub fn get_metadata_mut(&mut self) -> &mut HashMap<String, String> {
        match self {
            NodeData::Repository { metadata, .. } => metadata,
            NodeData::Subsystem { metadata, .. } => metadata,
            NodeData::Module { metadata, .. } => metadata,
            NodeData::File { metadata, .. } => metadata,
            NodeData::Symbol { metadata, .. } => metadata,
            NodeData::Package { metadata, .. } => metadata,
            NodeData::BuildTarget { metadata, .. } => metadata,
            NodeData::Author { metadata, .. } => metadata,
            NodeData::Owner { metadata, .. } => metadata,
            NodeData::Document { metadata, .. } => metadata,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum EdgeData {
    Contains,                // Hierarchy: Repos -> Subsystem -> Module -> File -> Symbol
    Calls,                   // Symbol A calls Symbol B
    References,              // Symbol A references Symbol B (generic)
    Imports,                 // File A imports File B
    DependsOnExternal,       // Package/File depends on external library
    AuthoredBy,              // Symbol/File authored by Author
    OwnedBy,                 // File/Subsystem owned by Owner
    ExecutedAtRuntime,       // Runtime connection
    SemanticSimilarity(f64), // Semantic similarity between symbols
    Describes,               // Document describes a Repo/Subsystem
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ProjectGraph {
    pub graph: StableGraph<NodeData, EdgeData, Directed>,
    #[allow(dead_code)]
    pub node_index: HashMap<String, NodeIndex>, // Fast lookup by "type:identifier"
    #[serde(skip)]
    pub reverse_index: HashMap<NodeIndex, String>, // Fast reverse lookup
}

#[allow(dead_code)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct NodeProjection {
    pub id: String,
    pub data: NodeData,
    pub children: Vec<String>,     // IDs of contained nodes
    pub dependencies: Vec<String>, // IDs of external dependency nodes
}

impl ProjectGraph {
    pub fn empty() -> Self {
        ProjectGraph {
            graph: StableGraph::new(),
            node_index: HashMap::new(),
            reverse_index: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn project_view(&self, from_node_id: &str, depth: usize) -> Vec<NodeProjection> {
        let mut projections = Vec::new();

        if let Some(&start_idx) = self.node_index.get(from_node_id) {
            self.build_projection_recursive(start_idx, depth, &mut projections);
        }

        projections
    }

    fn build_projection_recursive(
        &self,
        node_idx: NodeIndex,
        depth: usize,
        projections: &mut Vec<NodeProjection>,
    ) {
        let mut children = Vec::new();
        let mut dependencies = Vec::new();

        // Identify children (Contains edges)
        for edge in self.graph.edges_directed(node_idx, petgraph::Outgoing) {
            match &self.graph[edge.id()] {
                EdgeData::Contains => {
                    let child_idx = edge.target();
                    let child_id = self.get_node_id(child_idx);
                    children.push(child_id);

                    if depth > 0 {
                        self.build_projection_recursive(child_idx, depth - 1, projections);
                    }
                }
                EdgeData::Calls | EdgeData::Imports | EdgeData::DependsOnExternal => {
                    dependencies.push(self.get_node_id(edge.target()));
                }
                _ => {}
            }
        }

        projections.push(NodeProjection {
            id: self.get_node_id(node_idx),
            data: self.graph[node_idx].clone(),
            children,
            dependencies,
        });
    }

    pub fn get_node_id(&self, idx: NodeIndex) -> String {
        self.reverse_index
            .get(&idx)
            .cloned()
            .unwrap_or_else(|| format!("unknown:{:?}", idx))
    }

    pub fn get_symbol_references(&self, name: &str) -> Option<usize> {
        for node_idx in self.graph.node_indices() {
            if let NodeData::Symbol {
                name: s_name,
                references,
                ..
            } = &self.graph[node_idx]
                && s_name == name
            {
                return Some(*references);
            }
        }
        None
    }
}
