use serde::{Deserialize, Serialize};

use crate::app::input::ad::AdAccount;
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
    /// Active Directory account from the `[ad]` config section. Its username
    /// is the last entry of `users`; selecting it logs in with the account's
    /// password instead of an SSH key.
    pub ad: Option<AdAccount>,
    /// User to return to when the `a` key leaves the AD account.
    previous_user: Option<String>,
}

impl SshUsers {
    pub fn load() -> Self {
        let config: Option<Config> = load_config();
        Self::build(config.map(|config| config.users), AdAccount::load())
    }

    fn build(config: Option<UserConfig>, ad: Option<AdAccount>) -> Self {
        let mut users = vec!["ec2-user".to_string(), "ubuntu".to_string()];
        let config = config.unwrap_or_default();

        // Add additional users from config
        for user in config.additional_users {
            if !users.contains(&user) {
                users.push(user);
            }
        }

        // The AD account goes last, and is an AD login even if the same name
        // was also listed as a plain user.
        if let Some(ad) = &ad {
            users.retain(|user| user != &ad.username);
            users.push(ad.username.clone());
        }

        // Set default user from config if specified
        let default_user = config
            .default_user
            .filter(|configured| users.contains(configured));

        // If no default user from config, use first user
        let selected_user = default_user.or_else(|| users.first().cloned());

        SshUsers {
            users,
            selected_user,
            ad,
            previous_user: None,
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

    /// The AD account, if it is the selected user.
    pub fn selected_ad(&self) -> Option<&AdAccount> {
        self.ad
            .as_ref()
            .filter(|ad| self.selected_user.as_ref() == Some(&ad.username))
    }

    /// Jumps to the AD account, or back to the user selected before it.
    /// Returns false when no AD account is configured.
    pub fn toggle_ad(&mut self) -> bool {
        let Some(ad_user) = self.ad.as_ref().map(|ad| ad.username.clone()) else {
            return false;
        };
        if self.selected_user.as_ref() == Some(&ad_user) {
            self.selected_user = self
                .previous_user
                .take()
                .or_else(|| self.users.iter().find(|user| **user != ad_user).cloned())
                .or(Some(ad_user));
        } else {
            self.previous_user = self.selected_user.replace(ad_user);
        }
        true
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

#[cfg(test)]
mod tests {
    use super::*;

    const AD_USER: &str = "jane.doe@corp.example.local";

    fn ad() -> Option<AdAccount> {
        Some(AdAccount {
            username: AD_USER.to_string(),
            has_password: true,
        })
    }

    fn config(default_user: Option<&str>, additional: &[&str]) -> Option<UserConfig> {
        Some(UserConfig {
            default_user: default_user.map(str::to_string),
            additional_users: additional.iter().map(|user| user.to_string()).collect(),
        })
    }

    #[test]
    fn without_ad_nothing_changes() {
        let mut users = SshUsers::build(config(Some("admin"), &["admin"]), None);
        assert_eq!(users.users, ["ec2-user", "ubuntu", "admin"]);
        assert_eq!(users.selected_user.as_deref(), Some("admin"));
        assert!(users.selected_ad().is_none());
        assert!(!users.toggle_ad());
        assert_eq!(users.selected_user.as_deref(), Some("admin"));
    }

    #[test]
    fn ad_account_is_listed_last_and_not_selected_by_default() {
        let users = SshUsers::build(None, ad());
        assert_eq!(users.users, ["ec2-user", "ubuntu", AD_USER]);
        assert_eq!(users.selected_user.as_deref(), Some("ec2-user"));
        assert!(users.selected_ad().is_none());
    }

    #[test]
    fn default_user_can_be_the_ad_account() {
        let users = SshUsers::build(config(Some(AD_USER), &[]), ad());
        assert_eq!(
            users.selected_ad().map(|ad| ad.username.as_str()),
            Some(AD_USER)
        );
    }

    #[test]
    fn ad_account_also_listed_as_plain_user_appears_once() {
        let users = SshUsers::build(config(None, &[AD_USER, "admin"]), ad());
        assert_eq!(users.users, ["ec2-user", "ubuntu", "admin", AD_USER]);
    }

    #[test]
    fn cycling_reaches_the_ad_account() {
        let mut users = SshUsers::build(None, ad());
        users.next();
        users.next();
        assert!(users.selected_ad().is_some());
        users.next();
        assert_eq!(users.selected_user.as_deref(), Some("ec2-user"));
        users.previous();
        assert!(users.selected_ad().is_some());
    }

    #[test]
    fn toggle_jumps_to_ad_and_back_to_the_previous_user() {
        let mut users = SshUsers::build(None, ad());
        users.next(); // ubuntu
        assert!(users.toggle_ad());
        assert!(users.selected_ad().is_some());
        assert!(users.toggle_ad());
        assert_eq!(users.selected_user.as_deref(), Some("ubuntu"));
    }

    #[test]
    fn toggle_from_ad_as_default_goes_to_the_first_plain_user() {
        let mut users = SshUsers::build(config(Some(AD_USER), &[]), ad());
        assert!(users.toggle_ad());
        assert_eq!(users.selected_user.as_deref(), Some("ec2-user"));
        assert!(users.toggle_ad());
        assert!(users.selected_ad().is_some());
    }
}
