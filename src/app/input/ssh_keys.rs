use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SshKeys {
    pub keys: Vec<String>,
    pub selected_key: Option<String>,
}

impl SshKeys {
    pub fn load() -> Self {
        let keys = Self::get_ssh_private_keys();
        let selected_key = keys.first().cloned();

        SshKeys { keys, selected_key }
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
                {
                    continue;
                }

                // Check if it's likely a private key by reading first line
                if Self::is_private_key(&path) {
                    keys.push(filename);
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
