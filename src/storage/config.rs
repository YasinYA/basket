use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub theme_index: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self { theme_index: 0 }
    }
}

fn config_path() -> String {
    if let Ok(path) = env::var("BASKET_CONFIG_PATH") {
        return path;
    }
    let home = env::var("HOME").unwrap_or_else(|_| "/home/unknown".to_string());
    format!("{}/.basket_config.json", home)
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    let Ok(contents) = fs::read_to_string(&path) else {
        return AppConfig::default();
    };

    serde_json::from_str(&contents).unwrap_or_default()
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = config_path();
    let payload =
        serde_json::to_string_pretty(config).map_err(|err| format!("serialize: {}", err))?;

    if let Some(parent) = Path::new(&path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|err| format!("create config dir: {}", err))?;
        }
    }

    fs::write(&path, payload).map_err(|err| format!("write config: {}", err))?;
    Ok(())
}
