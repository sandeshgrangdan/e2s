use serde::de::DeserializeOwned;
use std::path::PathBuf;
use std::{env, fs};

pub fn load_config<T: DeserializeOwned>() -> Option<T> {
    let config_path = get_config_path()?;

    if !config_path.exists() {
        return None;
    }

    let content = fs::read_to_string(config_path).ok()?;
    toml::from_str(&content).ok()
}

pub fn get_config_path() -> Option<PathBuf> {
    if let Ok(custom) = env::var("E2S_CONFIG") {
        // Expand tilde if present
        let path = if custom.starts_with("~/") {
            let home = dirs::home_dir()?;
            home.join(&custom[2..]) // Skip the "~/" part
        } else if custom == "~" {
            dirs::home_dir()?
        } else {
            PathBuf::from(custom)
        };

        return Some(path);
    }

    let mut path = dirs::home_dir()?;
    path.push(".config");
    path.push("e2s");
    path.push("config.toml");
    Some(path)
}
