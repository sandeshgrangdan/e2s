//! The `[ad]` config section: an Active Directory account to log in with a
//! password instead of an SSH key.
//!
//! ```toml
//! [ad]
//! username = "jane.doe@corp.example.local"
//! password = 'secret'   # optional
//! ```

use serde::Deserialize;

use crate::utils::config::load_config;

#[derive(Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    ad: AdConfig,
}

// Deliberately not `Debug`: it holds the password.
#[derive(Deserialize, Default)]
struct AdConfig {
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    password: Option<String>,
}

/// The account as the TUI needs to know it. The password itself stays in the
/// config file and is only read at the moment ssh asks for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdAccount {
    pub username: String,
    pub has_password: bool,
}

impl AdAccount {
    pub fn load() -> Option<Self> {
        Self::from_config(load_config()?)
    }

    fn from_config(config: Config) -> Option<Self> {
        let username = config.ad.username?.trim().to_string();
        if username.is_empty() {
            return None;
        }
        Some(Self {
            username,
            has_password: config.ad.password.is_some_and(|p| !p.is_empty()),
        })
    }
}

/// The saved password, if there is one.
pub fn stored_password() -> Option<String> {
    let config: Config = load_config()?;
    config.ad.password.filter(|password| !password.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(toml: &str) -> Option<AdAccount> {
        AdAccount::from_config(toml::from_str(toml).unwrap())
    }

    #[test]
    fn no_section_means_no_account() {
        assert_eq!(account(""), None);
        assert_eq!(account("[users]\ndefault_user = \"ubuntu\"\n"), None);
        assert_eq!(account("[ad]\npassword = 'x'\n"), None);
        assert_eq!(account("[ad]\nusername = \"  \"\n"), None);
    }

    #[test]
    fn username_alone_is_enough() {
        assert_eq!(
            account("[ad]\nusername = \"jane.doe@corp.example.local\"\n"),
            Some(AdAccount {
                username: "jane.doe@corp.example.local".into(),
                has_password: false,
            })
        );
    }

    #[test]
    fn password_is_noticed_but_not_kept() {
        let found = account("[ad]\nusername = 'CORP\\jane'\npassword = 'p@ss \"w\\rd'\n").unwrap();
        assert_eq!(found.username, r"CORP\jane");
        assert!(found.has_password);
        assert!(!format!("{found:?}").contains("p@ss"));
    }

    #[test]
    fn empty_password_counts_as_none() {
        assert!(
            !account("[ad]\nusername = 'jane'\npassword = ''\n")
                .unwrap()
                .has_password
        );
    }
}
