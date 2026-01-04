use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::utils::config::load_config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub keys: KeyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyConfig {
    #[serde(default)]
    pub default_key: Option<String>,
    #[serde(default)]
    pub additional_keys: Vec<String>,
}

impl Default for KeyConfig {
    fn default() -> Self {
        KeyConfig {
            default_key: None,
            additional_keys: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SshKeys {
    pub keys: Vec<String>,
    pub selected_key: Option<String>,
}

impl SshKeys {
    pub fn load() -> Self {
        let mut keys = Self::get_ssh_private_keys();
        let mut default_key: Option<String> = None;

        let config: Option<Config> = load_config();

        // Try to load config from ~/.config/e2s/config.toml
        if let Some(config) = config {
            // Add additional keys from config (full paths)
            for key_path in config.keys.additional_keys {
                // Expand home directory if path starts with ~
                let expanded_path = Self::expand_home_dir(&key_path);

                // Verify the key file exists and is a private key
                if let Ok(path) = PathBuf::from(&expanded_path).canonicalize() {
                    if path.is_file() && Self::is_private_key(&path) {
                        let key_entry = expanded_path.clone();
                        if !keys.contains(&key_entry) {
                            keys.push(key_entry);
                        }
                    }
                }
            }

            // Set default key from config if specified
            if let Some(configured_default) = config.keys.default_key {
                // Check if it's a filename (from .ssh) or full path
                if configured_default.contains('/') || configured_default.contains('\\') {
                    // It's a full path
                    let expanded_path = Self::expand_home_dir(&configured_default);
                    if keys.contains(&expanded_path) {
                        default_key = Some(expanded_path);
                    }
                } else {
                    // It's just a filename, find the matching full path in keys
                    for key in &keys {
                        if let Some(filename) = PathBuf::from(key).file_name() {
                            if filename.to_string_lossy() == configured_default {
                                default_key = Some(key.clone());
                                break;
                            }
                        }
                    }
                }
            }
        }

        // If no default key from config, use first key
        let selected_key = default_key.or_else(|| keys.first().cloned());

        SshKeys { keys, selected_key }
    }

    fn expand_home_dir(path: &str) -> String {
        if path.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                return path.replacen("~", &home.to_string_lossy(), 1);
            }
        }
        path.to_string()
    }

    /// Move to the next key (wraps around to the beginning)
    pub fn next(&mut self) {
        if self.keys.is_empty() {
            return;
        }

        let current_index = self.get_current_index();
        let next_index = (current_index + 1) % self.keys.len();
        self.selected_key = self.keys.get(next_index).cloned();
    }

    /// Move to the previous key (wraps around to the end)
    pub fn previous(&mut self) {
        if self.keys.is_empty() {
            return;
        }

        let current_index = self.get_current_index();
        let prev_index = if current_index == 0 {
            self.keys.len() - 1
        } else {
            current_index - 1
        };
        self.selected_key = self.keys.get(prev_index).cloned();
    }

    /// Get the index of the currently selected key
    fn get_current_index(&self) -> usize {
        if let Some(ref selected) = self.selected_key {
            self.keys
                .iter()
                .position(|key| key == selected)
                .unwrap_or(0)
        } else {
            0
        }
    }

    /// Check if there are any keys
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Get total number of keys
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    fn get_ssh_private_keys() -> Vec<String> {
        let ssh_dir = match dirs::home_dir() {
            Some(mut path) => {
                path.push(".ssh");
                path
            }
            None => return Vec::new(),
        };

        if !ssh_dir.exists() || !ssh_dir.is_dir() {
            return Vec::new();
        }

        let mut keys = Vec::new();

        if let Ok(entries) = fs::read_dir(&ssh_dir) {
            for entry in entries.flatten() {
                let path = entry.path();

                // Skip if it's not a file
                if !path.is_file() {
                    continue;
                }

                // Get filename
                let filename = match path.file_name() {
                    Some(name) => name.to_string_lossy().to_string(),
                    None => continue,
                };

                // Skip .pub files and known_hosts, config, etc.
                if filename.ends_with(".pub")
                    || filename == "known_hosts"
                    || filename == "config"
                    || filename == "authorized_keys"
                    || filename.starts_with('.')
                // Skip hidden files like .ssh
                {
                    continue;
                }

                // Check if it's likely a private key by reading first line
                if Self::is_private_key(&path) {
                    // Store the full path, not just the filename
                    if let Some(full_path) = path.to_str() {
                        keys.push(full_path.to_string());
                    }
                }
            }
        }

        keys.sort();
        keys
    }

    fn is_private_key(path: &PathBuf) -> bool {
        if let Ok(content) = fs::read_to_string(path) {
            let first_line = content.lines().next().unwrap_or("");
            first_line.contains("BEGIN") && first_line.contains("PRIVATE KEY")
        } else {
            false
        }
    }
}
