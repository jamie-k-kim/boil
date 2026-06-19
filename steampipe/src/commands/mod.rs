pub mod ls_show;
pub mod find_expand;
pub mod edit;
pub mod distill;
pub mod batch;

pub fn run_status(batch: &crate::batch::Batch, json: bool) -> anyhow::Result<String> {
    let layers_dir = batch.root.join("layers");
    let mut actual_layers = Vec::new();
    if let Ok(entries) = std::fs::read_dir(layers_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                if let Ok(name) = entry.file_name().into_string() {
                    actual_layers.push(name);
                }
            }
        }
    }
    actual_layers.sort();

    if json {
        let status = serde_json::json!({
            "batch_path": batch.root.display().to_string(),
            "source": batch.manifest.source,
            "created": batch.manifest.created,
            "layers": actual_layers
        });
        Ok(serde_json::to_string_pretty(&status)?)
    } else {
        let mut out = String::new();
        out.push_str(&format!("{}\n", console::style("--- Batch Status ---").bold().cyan()));
        out.push_str(&format!("{:<10} {}\n", console::style("Path:").dim(), batch.root.display()));
        out.push_str(&format!("{:<10} {}\n", console::style("Source:").dim(), batch.manifest.source));
        out.push_str(&format!("{:<10} {}\n", console::style("Created:").dim(), batch.manifest.created));
        out.push_str(&format!("{:<10} {}\n", console::style("Layers:").dim(), actual_layers.join(", ")));
        Ok(out)
    }
}
