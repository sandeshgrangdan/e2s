use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub users: UserConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    #[serde(default)]
    pub default_user: Option<String>,
    #[serde(default)]
    pub additional_users: Vec<String>,
}

impl Default for UserConfig {
    fn default() -> Self {
        UserConfig {
            default_user: None,
            additional_users: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SshUsers {
    pub users: Vec<String>,
    pub selected_user: Option<String>,
}

impl SshUsers {
    pub fn load() -> Self {
        let mut users = vec!["ec2-user".to_string(), "ubuntu".to_string()];
        let mut default_user: Option<String> = None;

        // Try to load config from ~/.config/ec2/config.toml
        if let Some(config) = Self::load_config() {
            // Add additional users from config
            for user in config.users.additional_users {
                if !users.contains(&user) {
                    users.push(user);
                }
            }

            // Set default user from config if specified
            if let Some(configured_default) = config.users.default_user {
                if users.contains(&configured_default) {
                    default_user = Some(configured_default);
                }
            }
        }

        // If no default user from config, use first user
        let selected_user = default_user.or_else(|| users.first().cloned());

        SshUsers {
            users,
            selected_user,
        }
    }

    fn load_config() -> Option<Config> {
        let config_path = Self::get_config_path()?;

        if !config_path.exists() {
            return None;
        }

        let content = fs::read_to_string(config_path).ok()?;
        toml::from_str(&content).ok()
    }

    fn get_config_path() -> Option<PathBuf> {
        let mut path = dirs::home_dir()?;
        path.push(".config");
        path.push("e2s");
        path.push("config.toml");
        Some(path)
    }

    /// Move to the next user (wraps around to the beginning)
    pub fn next(&mut self) {
        if self.users.is_empty() {
            return;
        }

        let current_index = self.get_current_index();
        let next_index = (current_index + 1) % self.users.len();
        self.selected_user = self.users.get(next_index).cloned();
    }

    /// Move to the previous user (wraps around to the end)
    pub fn previous(&mut self) {
        if self.users.is_empty() {
            return;
        }

        let current_index = self.get_current_index();
        let prev_index = if current_index == 0 {
            self.users.len() - 1
        } else {
            current_index - 1
        };
        self.selected_user = self.users.get(prev_index).cloned();
    }

    /// Get the index of the currently selected user
    fn get_current_index(&self) -> usize {
        if let Some(ref selected) = self.selected_user {
            self.users
                .iter()
                .position(|user| user == selected)
                .unwrap_or(0)
        } else {
            0
        }
    }

    /// Check if there are any users
    pub fn is_empty(&self) -> bool {
        self.users.is_empty()
    }

    /// Get total number of users
    pub fn len(&self) -> usize {
        self.users.len()
    }

    /// Create a sample config file
    pub fn create_sample_config() -> std::io::Result<()> {
        let config_path = Self::get_config_path().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Home directory not found")
        })?;

        // Create directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let sample_config = Config {
            users: UserConfig {
                default_user: Some("ec2-user".to_string()),
                additional_users: vec!["admin".to_string(), "root".to_string()],
            },
        };

        let toml_string = toml::to_string_pretty(&sample_config)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        fs::write(config_path, toml_string)?;
        Ok(())
    }
}

