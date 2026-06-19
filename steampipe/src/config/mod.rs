use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use anyhow::Result;
use std::fs;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Config {
    pub last_batch: Option<PathBuf>,
}

const CONFIG_FILENAME: &str = "boil.toml";

fn get_config_path() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(CONFIG_FILENAME)
}

pub fn load_config() -> Result<Config> {
    let path = get_config_path();
    if !path.exists() {
        return Ok(Config::default());
    }
    let content = fs::read_to_string(path)?;
    
    #[derive(Deserialize)]
    struct BoilToml {
        state: Option<Config>,
    }
    
    if let Ok(bt) = toml::from_str::<BoilToml>(&content) {
        if let Some(state) = bt.state {
            return Ok(state);
        }
    }
    
    Ok(Config::default())
}

pub fn save_config(config: &Config) -> Result<()> {
    use toml_edit::{DocumentMut, value};
    let path = get_config_path();
    
    let mut doc = if path.exists() {
        let content = fs::read_to_string(&path)?;
        content.parse::<DocumentMut>().unwrap_or_default()
    } else {
        DocumentMut::new()
    };

    if let Some(ref last_batch) = config.last_batch {
        doc["state"]["last_batch"] = value(last_batch.to_string_lossy().to_string());
    } else {
        if let Some(state) = doc.get_mut("state") {
            if let Some(tbl) = state.as_table_mut() {
                tbl.remove("last_batch");
                if tbl.is_empty() {
                    doc.remove("state");
                }
            } else if let Some(tbl) = state.as_inline_table_mut() {
                tbl.remove("last_batch");
                if tbl.is_empty() {
                    doc.remove("state");
                }
            }
        }
    }

    fs::write(path, doc.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_save_config() {
        let temp = tempfile::tempdir().unwrap();
        unsafe {
            std::env::set_var("HOME", temp.path());
        }

        // 1. Initial load should return default empty config
        let cfg = load_config().unwrap();
        assert!(cfg.last_batch.is_none());

        // 2. Save config
        let mut cfg = Config::default();
        cfg.last_batch = Some(PathBuf::from("/some/path"));
        save_config(&cfg).unwrap();

        // 3. Reload config and verify
        let loaded = load_config().unwrap();
        assert_eq!(loaded.last_batch, Some(PathBuf::from("/some/path")));

        // 4. Test preservation of other tables and comments
        let custom_toml = r#"[default]
ignore = ["target"] # user comment here

[state]
last_batch = "/some/path"
"#;
        fs::write(get_config_path(), custom_toml).unwrap();

        let mut cfg2 = Config::default();
        cfg2.last_batch = Some(PathBuf::from("/another/path"));
        save_config(&cfg2).unwrap();

        let new_content = fs::read_to_string(get_config_path()).unwrap();
        assert!(new_content.contains("[default]"));
        assert!(new_content.contains("ignore = [\"target\"] # user comment here"));
        assert!(new_content.contains("last_batch = \"/another/path\""));
    }
}
