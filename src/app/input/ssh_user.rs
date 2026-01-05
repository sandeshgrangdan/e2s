use serde::{Deserialize, Serialize};

use crate::utils::config::load_config;

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

        let config: Option<Config> = load_config();

        // Try to load config from ~/.config/ec2/config.toml
        if let Some(config) = config {
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
}
