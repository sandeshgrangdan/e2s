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

/// Says where the config file is broken, if it exists but [`load_config`]
/// cannot read it as `T` - in that case the settings silently fall back to
/// their defaults, which is confusing without a hint.
///
/// Only the position is reported, never the offending text: the file may
/// contain a password.
pub fn config_problem<T: DeserializeOwned>() -> Option<String> {
    let path = get_config_path()?;
    let content = fs::read_to_string(&path).ok()?;
    let error = toml::from_str::<T>(&content).err()?;
    let line = error.span().map(|span| {
        let before = &content.as_bytes()[..span.start.min(content.len())];
        before.iter().filter(|byte| **byte == b'\n').count() + 1
    });
    let path = path.display();
    Some(match line {
        Some(line) => format!("Config error at line {line} of {path} - using defaults"),
        None => format!("Config error in {path} - using defaults"),
    })
}

/// The config file path as an absolute path, so that a process started from
/// another directory finds the same file.
pub fn absolute_config_path() -> Option<PathBuf> {
    let path = get_config_path()?;
    if path.is_absolute() {
        Some(path)
    } else {
        Some(env::current_dir().ok()?.join(path))
    }
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
