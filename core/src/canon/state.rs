use crate::canon::{FileInfo, ProjectGraph};
use anyhow::Result;
use std::path::Path;

#[derive(serde::Deserialize)]
pub struct CanonState {
    pub file_infos: Vec<FileInfo>,
    pub graph: ProjectGraph,
}

#[derive(serde::Serialize)]
pub struct CanonStateRef<'a> {
    pub file_infos: &'a [FileInfo],
    pub graph: &'a ProjectGraph,
}

impl<'a> CanonStateRef<'a> {
    pub fn new(file_infos: &'a [FileInfo], graph: &'a ProjectGraph) -> Self {
        Self { file_infos, graph }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let file = std::fs::File::create(path)?;
        let writer = std::io::BufWriter::new(file);
        bincode::serialize_into(writer, self)?;
        Ok(())
    }
}

impl CanonState {
    pub fn load(path: &Path) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        let mut state: Self = bincode::deserialize_from(reader)?;

        for (id, &idx) in &state.graph.node_index {
            state.graph.reverse_index.insert(idx, id.clone());
        }

        Ok(state)
    }
}
