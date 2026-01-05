use serde::Deserialize;
use std::io;
use std::process::Command;

use crate::utils::config::load_config;

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Config {
    #[serde(default)]
    pub terminal: TerminalConfig,
    // ... other config fields
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct TerminalConfig {
    pub emulator: Option<String>,
    pub shell: Option<String>,
}

impl Config {
    pub fn load() -> Self {
        let config: Option<Config> = load_config();

        if let Some(cfg) = config {
            cfg
        } else {
            eprintln!("No terminal config found, using defaults");
            Config::default()
        }
    }

    /// Get terminal emulator from config or auto-detect
    pub fn get_terminal_emulator(&self) -> io::Result<String> {
        if let Some(emulator) = &self.terminal.emulator {
            // Validate that the specified emulator exists
            if Self::is_terminal_available(emulator) {
                return Ok(emulator.clone());
            } else {
                eprintln!(
                    "Warning: Configured terminal '{}' not found, attempting auto-detect",
                    emulator
                );
            }
        }

        // Auto-detect available terminal
        Self::detect_terminal()
    }

    fn is_terminal_available(emulator: &str) -> bool {
        Command::new("which")
            .arg(emulator)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn detect_terminal() -> io::Result<String> {
        let terminals = [
            "ghostty",        // Modern, fast GPU-accelerated
            "alacritty",      // Popular GPU-accelerated
            "kitty",          // Feature-rich GPU-accelerated
            "wezterm",        // GPU-accelerated with multiplexing
            "rio",            // Rust-based GPU-accelerated
            "foot",           // Lightweight Wayland
            "gnome-terminal", // GNOME default
            "konsole",        // KDE default
            "terminator",     // Multiple terminals
            "tilix",          // Tiling terminal
            "xterm",          // Classic fallback
        ];

        for terminal in &terminals {
            if Self::is_terminal_available(terminal) {
                return Ok(terminal.to_string());
            }
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No supported terminal emulator found. Please install one of: ghostty, alacritty, kitty, wezterm, gnome-terminal, konsole, terminator, tilix, xterm, foot, rio",
        ))
    }

    pub fn get_shell(&self) -> String {
        if let Some(shell) = &self.terminal.shell {
            return shell.clone();
        }

        // Detect from environment
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
    }
}
