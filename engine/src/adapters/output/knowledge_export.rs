use boil_core::canon::{FileInfo, ProjectGraph};
use crate::core::engine::EngineConfig;
use crate::ports::{ExportProvider, OutputModule};
use anyhow::Result;
use std::path::Path;

pub struct KnowledgeExportModule {
    exporters: Vec<Box<dyn ExportProvider>>,
}

impl KnowledgeExportModule {
    pub fn new(exporters: Vec<Box<dyn ExportProvider>>) -> Self {
        Self { exporters }
    }
}

impl OutputModule for KnowledgeExportModule {
    fn export(
        &self,
        _project_root: &Path,
        output_root: &Path,
        config: &EngineConfig,
        _file_infos: &mut Vec<FileInfo>,
        graph: &mut ProjectGraph,
    ) -> Result<()> {
        let timestamp = config
            .force_timestamp
            .clone()
            .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string());

        let prefix = "canon";
        let boil_root = output_root.join(format!("{}_{}", prefix, timestamp));

        std::fs::create_dir_all(&boil_root)?;

        // Export binary state for Steampipe
        let state_ref = boil_core::canon::state::CanonStateRef::new(_file_infos, graph);
        state_ref.save(&boil_root.join("canon.bin"))?;
        if !config.silent {
            println!(
                "  Exported binary canon to {}/canon.bin",
                boil_root.display()
            );
        }

        for exporter in &self.exporters {
            exporter.export(graph, &boil_root)?;
            if !config.silent {
                println!(
                    "  Exported {} canon to {}/canon.{}",
                    exporter.format_name(),
                    boil_root.display(),
                    exporter.extension()
                );
            }
        }

        Ok(())
    }
}
