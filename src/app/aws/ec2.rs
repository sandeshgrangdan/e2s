use std::{io, process::{Stdio, Command}};

use aws_config::{
    meta::region::RegionProviderChain, profile::ProfileFileRegionProvider, BehaviorVersion, Region,
    SdkConfig,
};
use fakeit::internet;
use itertools::Itertools;
use rand::{seq::SliceRandom, Rng};
use serde::{Deserialize, Serialize};

use aws_sdk_ec2::Client;

use crate::app::aws;
use crate::app::App;

// ANCHOR: application
#[derive(Debug, Clone)]
pub enum Ec2Client {
    Client(Client),
    None,
}

#[derive(Debug, Clone)]
pub struct Data {
    pub name: String,
    pub instance_id: String,
    pub ami_id: String,
    pub key_group: String,
    pub public_ipv4: String,
    pub private_ipv4: String,
    pub status: String,
}

impl Data {
    pub const fn ref_array(&self) -> [&String; 7] {
        [
            &self.name,
            &self.instance_id,
            &self.ami_id,
            &self.key_group,
            &self.public_ipv4,
            &self.public_ipv4,
            &self.status,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectMode {
    Public,
    Private,
    Ssm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectModeConfig {
    pub connect_mode: Option<String>,
}

impl ConnectModeConfig {
    pub fn load() -> ConnectMode {
        // Load the full config
        let config: Option<ConnectModeConfig> = crate::utils::config::load_config();

        if let Some(cfg) = config {
            if let Some(mode) = cfg.connect_mode {
                match mode.to_lowercase().as_str() {
                    "public" => ConnectMode::Public,
                    "private" => ConnectMode::Private,
                    "ssm" => ConnectMode::Ssm,
                    _ => ConnectMode::Private,
                }
            } else {
                eprintln!("No Connect Mode config found, using defaults");
                ConnectMode::Private
            }
        } else {
            eprintln!("No Connect Mode config found, using defaults");
            ConnectMode::Private
        }
    }
}

impl ConnectMode {
    pub fn next(self) -> Self {
        match self {
            ConnectMode::Public => ConnectMode::Private,
            ConnectMode::Private => ConnectMode::Ssm,
            ConnectMode::Ssm => ConnectMode::Public,
        }
    }

    pub fn toggle(&mut self) {
        *self = match *self {
            ConnectMode::Public => ConnectMode::Private,
            ConnectMode::Private => ConnectMode::Ssm,
            ConnectMode::Ssm => ConnectMode::Public,
        };
    }
}

fn generate_instance_id() -> String {
    let hex: String = (0..17)
        .map(|_| format!("{:x}", rand::random::<u8>() % 16))
        .collect();
    format!("i-{}", hex)
}

fn generate_ami_id() -> String {
    let hex: String = (0..17)
        .map(|_| format!("{:x}", rand::random::<u8>() % 16))
        .collect();
    format!("ami-{}", hex)
}

pub fn generate_fake_instances() -> Vec<Data> {
    let mut rng = rand::thread_rng();

    let envs = ["prod", "qa", "dev", "staging"];
    let roles = ["web", "api", "worker", "db"];
    let statuses = ["running", "stopped", "terminated"];
    let regions = ["ap-northeast-1", "us-east-1", "eu-west-1"];

    (1..=300)
        .map(|i| {
            let env = envs.choose(&mut rng).unwrap();
            let role = roles.choose(&mut rng).unwrap();
            let region = regions.choose(&mut rng).unwrap();

            let name = format!("{env}-{role}-{:02}", i % 20 + 1);

            let instance_id = generate_instance_id();
            let ami_id = generate_ami_id();
            let key_group = format!("{env}-keypair-{region}");

            let status = statuses.choose(&mut rng).unwrap().to_string();

            let private_ipv4 = format!(
                "10.{}.{}.{}",
                rng.gen_range(0..255),
                rng.gen_range(0..255),
                rng.gen_range(10..250)
            );

            let public_ipv4 = internet::ipv4_address();

            Data {
                name,
                instance_id,
                ami_id,
                key_group,
                public_ipv4,
                private_ipv4,
                status,
            }
        })
        .sorted_by(|a, b| a.name.cmp(&b.name))
        .collect()
}

pub async fn get_config(profile: String, region: String) -> SdkConfig {
    let default_region = "us-east-1";
    if profile == *"None" {
        aws_config::defaults(BehaviorVersion::latest())
            .region(if region != *"None" {
                RegionProviderChain::first_try(Region::new(region))
                    .or_default_provider()
                    .or_else(Region::new(default_region))
            } else {
                RegionProviderChain::default_provider().or_else(Region::new(default_region))
            })
            .load()
            .await
    } else {
        aws_config::defaults(BehaviorVersion::latest())
            .region(if region == *"None" {
                RegionProviderChain::first_try(
                    ProfileFileRegionProvider::builder()
                        .profile_name(profile.clone())
                        .build(),
                )
                .or_default_provider()
                .or_else(Region::new(default_region))
            } else {
                RegionProviderChain::first_try(Region::new(region))
                    .or_default_provider()
                    .or_else(Region::new(default_region))
            })
            .profile_name(profile)
            .load()
            .await
    }
}

pub async fn fetch_instances(client: &Client) -> Result<Vec<Data>, Box<dyn std::error::Error>> {
    let mut instances: Vec<Data> = Vec::new();

    let mut paginator = client.describe_instances().into_paginator().send();

    while let Some(page) = paginator.next().await {
        let resp = page?;

        for reservation in resp.reservations() {
            for inst in reservation.instances() {
                let name = inst
                    .tags()
                    .iter()
                    .find(|t| t.key() == Some("Name"))
                    .and_then(|t| t.value())
                    .unwrap_or("-")
                    .to_string();

                instances.push(Data {
                    name,
                    instance_id: inst.instance_id().unwrap_or("-").to_string(),
                    ami_id: inst.image_id().unwrap_or("-").to_string(),
                    key_group: inst.key_name().unwrap_or("-").to_string(),
                    public_ipv4: inst.public_ip_address().unwrap_or("-").to_string(),
                    private_ipv4: inst.private_ip_address().unwrap_or("-").to_string(),
                    status: inst
                        .state()
                        .and_then(|s| s.name())
                        .map(|n| n.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                });
            }
        }
    }

    Ok(instances)
}
impl App {
    pub fn ec2_next(&mut self) {
        if self.display_items.is_empty() {
            self.state.select(None); // nothing to select
        } else {
            let i = match self.state.selected() {
                Some(i) => {
                    if i >= self.display_items.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.state.select(Some(i));
        }
    }

    pub fn ec2_previous(&mut self) {
        if self.display_items.is_empty() {
            self.state.select(None); // nothing to select
        } else {
            let i = match self.state.selected() {
                Some(i) => {
                    if i == 0 {
                        self.display_items.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.state.select(Some(i));
        }
    }
    pub async fn set_ec2_client(&mut self) {
        let config = get_config(self.args.profile.clone(), self.args.region.clone()).await;
        let client = Client::new(&config);
        self.ec2_client = Ec2Client::Client(client);
    }

    pub async fn fetch_ec2_data(&mut self) {
        if let Ec2Client::Client(client) = &self.ec2_client {
            match aws::ec2::fetch_instances(client).await {
                Ok(instances) => {
                    self.items = instances;
                }
                Err(err) => println!("{:?}", err),
            };
        }
    }

    pub fn set_ec2s(&mut self) {
        self.display_items = if self.search.1.input.is_empty() {
            self.items.to_vec()
        } else {
            let search_lower = self.search.1.input.to_lowercase();
            self.items
                .iter()
                .filter(|item| {
                    item.name.to_lowercase().contains(&search_lower)
                        || item.instance_id.to_lowercase().contains(&search_lower)
                        || item.ami_id.to_lowercase().contains(&search_lower)
                        || item.key_group.to_lowercase().contains(&search_lower)
                        || item.public_ipv4.to_lowercase().contains(&search_lower)
                        || item.private_ipv4.to_lowercase().contains(&search_lower)
                        || item.status.to_lowercase().contains(&search_lower)
                })
                .cloned()
                .collect()
        };

        if let Some(selected) = self.state.selected() {
            if let Some(item) = self.display_items.get(selected) {
                self.selected_item = Some(item.clone());
            } else {
                self.selected_item = None;
            }
        };
    }

    pub async fn ssh(&mut self) -> io::Result<()> {
        if let Some(item) = &self.selected_item {
            let mut cmd = match self.mode {
                ConnectMode::Public | ConnectMode::Private => {
                    let key_path = match &self.ssh_keys.selected_key {
                        Some(key) => key,
                        None => {
                            eprintln!("No SSH key selected.");
                            return Ok(());
                        }
                    };
                    let user = self.ssh_user.selected_user.as_deref().unwrap_or("ec2-user");
                    let ip = match self.mode {
                        ConnectMode::Public => &item.public_ipv4,
                        ConnectMode::Private => &item.private_ipv4,
                        _ => unreachable!(),
                    };
                    let mut ssh_cmd = Command::new("ssh");
                    ssh_cmd.args(["-i", key_path]);
                    ssh_cmd.arg(format!("{}@{}", user, ip));
                    ssh_cmd
                }
                ConnectMode::Ssm => {
                    let mut ssm_cmd = Command::new("aws");
                    ssm_cmd.args(["ssm", "start-session", "--target", &item.instance_id]);
                    if self.args.region != *"None" {
                        ssm_cmd.args(["--region", &self.args.region]);
                    }
                    if self.args.profile != *"None" {
                        ssm_cmd.args(["--profile", &self.args.profile]);
                    }
                    ssm_cmd
                }
            };

            // Spawn the process and wait for it to complete
            let mut child = cmd
                .stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .spawn()?;

            let status = child.wait()?;

            if !status.success() {
                eprintln!("Failed to launch session.");
            }
        }
        Ok(())
    }

    pub async fn ssh_in_new_window(&self, emulator: &str) -> io::Result<()> {
        if let Some(item) = &self.selected_item {
            let ssh_command = match self.mode {
                ConnectMode::Public | ConnectMode::Private => {
                    let key_path = match &self.ssh_keys.selected_key {
                        Some(key) => key,
                        None => {
                            eprintln!("No SSH key selected.");
                            return Ok(());
                        }
                    };
                    let user = self.ssh_user.selected_user.as_deref().unwrap_or("ec2-user");
                    let ip = match self.mode {
                        ConnectMode::Public => &item.public_ipv4,
                        ConnectMode::Private => &item.private_ipv4,
                        _ => unreachable!(),
                    };
                    format!("ssh -i {} {}@{}", key_path, user, ip)
                }
                ConnectMode::Ssm => {
                    let mut cmd = format!("aws ssm start-session --target {}", item.instance_id);
                    if self.args.region != *"None" {
                        cmd.push_str(&format!(" --region {}", self.args.region));
                    }
                    if self.args.profile != *"None" {
                        cmd.push_str(&format!(" --profile {}", self.args.profile));
                    }
                    cmd
                }
            };

            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
            let shell_name = std::path::Path::new(&shell)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("bash");

            let exec_command = format!("{}; exec {}", ssh_command, shell);

            // Detect OS and WSL
            let is_macos = cfg!(target_os = "macos");
            let is_wsl = std::path::Path::new("/proc/version").exists()
                && std::fs::read_to_string("/proc/version")
                    .map(|s| {
                        s.to_lowercase().contains("microsoft") || s.to_lowercase().contains("wsl")
                    })
                    .unwrap_or(false);

            let mut command = match emulator.to_lowercase().as_str() {
                "iterm2" | "iterm" if is_macos => {
                    // macOS iTerm2 - use AppleScript
                    let applescript = format!(
                        r#"tell application "iTerm"
                            create window with default profile
                            tell current session of current window
                                write text "{}"
                            end tell
                        end tell"#,
                        exec_command.replace("\\", "\\\\").replace("\"", "\\\"")
                    );
                    let mut cmd = Command::new("osascript");
                    cmd.arg("-e").arg(&applescript);
                    cmd
                }
                "terminal" if is_macos => {
                    // macOS Terminal.app
                    let applescript = format!(
                        r#"tell application "Terminal"
                            do script "{}"
                            activate
                        end tell"#,
                        exec_command.replace("\\", "\\\\").replace("\"", "\\\"")
                    );
                    let mut cmd = Command::new("osascript");
                    cmd.arg("-e").arg(&applescript);
                    cmd
                }
                "ghostty" => {
                    if is_macos {
                        // Try CLI first
                        if Command::new("ghostty").arg("--version").output().is_ok() {
                            // CLI is available - use it directly
                            let mut cmd = Command::new("ghostty");
                            cmd.arg("-e").arg(&shell_name).arg("-c").arg(&exec_command);
                            cmd
                        } else {
                            // CLI not available - use AppleScript to control Ghostty
                            let applescript = format!(
                                r#"tell application "Ghostty"
                                    activate
                                end tell
                                delay 0.3
                                tell application "System Events"
                                    keystroke "t" using {{command down}}
                                    delay 0.2
                                    keystroke "{}"
                                    keystroke return
                                end tell"#,
                                exec_command.replace("\\", "\\\\").replace("\"", "\\\"")
                            );
                            let mut cmd = Command::new("osascript");
                            cmd.arg("-e").arg(&applescript);
                            cmd
                        }
                    } else if is_wsl {
                        // WSL - use Windows executable
                        let mut cmd = Command::new("ghostty.exe");
                        cmd.arg("-e").arg(&shell_name).arg("-c").arg(&exec_command);
                        cmd
                    } else {
                        // Linux - standard CLI
                        let mut cmd = Command::new("ghostty");
                        cmd.arg("-e").arg(&shell_name).arg("-c").arg(&exec_command);
                        cmd
                    }
                }
                "warp" if is_macos => {
                    // Warp on macOS - limited CLI support
                    let applescript = format!(
                        r#"tell application "Warp"
                            activate
                        end tell
                        delay 0.5
                        tell application "System Events"
                            keystroke "t" using {{command down}}
                            delay 0.2
                            keystroke "{}"
                            keystroke return
                        end tell"#,
                        exec_command.replace("\\", "\\\\").replace("\"", "\\\"")
                    );
                    let mut cmd = Command::new("osascript");
                    cmd.arg("-e").arg(&applescript);
                    cmd
                }
                "alacritty" => {
                    if is_wsl {
                        let mut cmd = Command::new("alacritty.exe");
                        cmd.arg("-e").arg(&shell_name).arg("-c").arg(&exec_command);
                        cmd
                    } else {
                        let mut cmd = Command::new("alacritty");
                        cmd.arg("-e").arg(&shell_name).arg("-c").arg(&exec_command);
                        cmd
                    }
                }
                "kitty" => {
                    if is_wsl {
                        let mut cmd = Command::new("kitty.exe");
                        cmd.arg(&shell_name).arg("-c").arg(&exec_command);
                        cmd
                    } else {
                        let mut cmd = Command::new("kitty");
                        cmd.arg(&shell_name).arg("-c").arg(&exec_command);
                        cmd
                    }
                }
                "wezterm" => {
                    if is_wsl {
                        // WSL - call Windows wezterm.exe
                        let mut cmd = Command::new("wezterm.exe");
                        cmd.arg("start")
                            .arg("--cwd")
                            .arg(".")
                            .arg("--")
                            .arg("wsl.exe")
                            .arg("-e")
                            .arg(&shell_name)
                            .arg("-c")
                            .arg(&exec_command);
                        cmd
                    } else if is_macos || cfg!(target_os = "linux") {
                        // macOS and Linux
                        let mut cmd = Command::new("wezterm");
                        cmd.arg("start")
                            .arg(&shell_name)
                            .arg("-c")
                            .arg(&exec_command);
                        cmd
                    } else {
                        let mut cmd = Command::new("wezterm");
                        cmd.arg("start")
                            .arg(&shell_name)
                            .arg("-c")
                            .arg(&exec_command);
                        cmd
                    }
                }
                "hyper" => {
                    if is_macos {
                        let mut cmd = Command::new("open");
                        cmd.arg("-a").arg("Hyper");
                        cmd
                    } else {
                        let mut cmd = Command::new("hyper");
                        cmd.arg("-e").arg(&exec_command);
                        cmd
                    }
                }
                "tabby" => {
                    if is_macos {
                        let mut cmd = Command::new("open");
                        cmd.arg("-a").arg("Tabby");
                        cmd
                    } else {
                        let mut cmd = Command::new("tabby");
                        cmd
                    }
                }
                "tilix" => {
                    // Linux only
                    let mut cmd = Command::new("tilix");
                    cmd.arg("-e").arg(&exec_command);
                    cmd
                }
                "terminator" => {
                    // Linux only
                    let mut cmd = Command::new("terminator");
                    cmd.arg("-e").arg(&exec_command);
                    cmd
                }
                "gnome-terminal" => {
                    // Linux only (mostly)
                    let mut cmd = Command::new("gnome-terminal");
                    cmd.arg("--").arg(&shell_name).arg("-c").arg(&exec_command);
                    cmd
                }
                "konsole" => {
                    // Linux only (KDE)
                    let mut cmd = Command::new("konsole");
                    cmd.arg("-e").arg(&shell_name).arg("-c").arg(&exec_command);
                    cmd
                }
                "xterm" => {
                    // Works on both, but primarily Linux
                    let mut cmd = Command::new("xterm");
                    cmd.arg("-e").arg(&shell_name).arg("-c").arg(&exec_command);
                    cmd
                }
                "rxvt" | "urxvt" => {
                    // Linux only
                    let mut cmd = Command::new(emulator);
                    cmd.arg("-e").arg(&shell_name).arg("-c").arg(&exec_command);
                    cmd
                }
                "st" => {
                    // Simple Terminal (Linux)
                    let mut cmd = Command::new("st");
                    cmd.arg("-e").arg(&shell_name).arg("-c").arg(&exec_command);
                    cmd
                }
                "foot" => {
                    // Wayland terminal (Linux)
                    let mut cmd = Command::new("foot");
                    cmd.arg(&shell_name).arg("-c").arg(&exec_command);
                    cmd
                }
                "windows-terminal" | "wt" if is_wsl => {
                    // Windows Terminal from WSL
                    let mut cmd = Command::new("wt.exe");
                    cmd.arg("--")
                        .arg("wsl.exe")
                        .arg("~")
                        .arg("-e")
                        .arg(&shell_name)
                        .arg("-l")
                        .arg("-c")
                        .arg(&ssh_command);
                    cmd
                }
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("Unknown or unsupported terminal emulator: {}", emulator),
                    ));
                }
            };

            command
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());

            let _ = command.spawn().map(|_| ()).map_err(|e| {
                eprintln!("Failed to open terminal '{}': {}", emulator, e);
                e
            });
        }
        Ok(())
    }
}
