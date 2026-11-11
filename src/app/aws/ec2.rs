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
use crate::app::connect::{ConnectSpec, SshAuth, Target};
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
    pub details: Details,
}

/// The rest of what EC2 reports about an instance, shown in the details
/// popup (`d`). Empty strings mean "not set".
#[derive(Debug, Clone, Default)]
pub struct Details {
    pub instance_type: String,
    pub architecture: String,
    pub platform: String,
    pub availability_zone: String,
    pub vpc_id: String,
    pub subnet_id: String,
    pub security_groups: Vec<String>,
    /// Name of the IAM instance profile (the role the instance runs as).
    pub iam_profile: String,
    pub launch_time: String,
    pub private_dns: String,
    pub public_dns: String,
    pub tags: Vec<(String, String)>,
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
                details: Details::default(),
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
                    details: Details {
                        instance_type: inst
                            .instance_type()
                            .map(|t| t.as_str().to_string())
                            .unwrap_or_default(),
                        architecture: inst
                            .architecture()
                            .map(|a| a.as_str().to_string())
                            .unwrap_or_default(),
                        platform: inst.platform_details().unwrap_or_default().to_string(),
                        availability_zone: inst
                            .placement()
                            .and_then(|p| p.availability_zone())
                            .unwrap_or_default()
                            .to_string(),
                        vpc_id: inst.vpc_id().unwrap_or_default().to_string(),
                        subnet_id: inst.subnet_id().unwrap_or_default().to_string(),
                        security_groups: inst
                            .security_groups()
                            .iter()
                            .map(|g| {
                                format!(
                                    "{} ({})",
                                    g.group_name().unwrap_or("-"),
                                    g.group_id().unwrap_or("-")
                                )
                            })
                            .collect(),
                        // arn:aws:iam::<account>:instance-profile/<name>
                        iam_profile: inst
                            .iam_instance_profile()
                            .and_then(|p| p.arn())
                            .map(|arn| arn.rsplit('/').next().unwrap_or(arn).to_string())
                            .unwrap_or_default(),
                        launch_time: inst
                            .launch_time()
                            .and_then(|t| {
                                t.fmt(aws_sdk_ec2::primitives::DateTimeFormat::DateTime)
                                    .ok()
                            })
                            .unwrap_or_default(),
                        private_dns: inst.private_dns_name().unwrap_or_default().to_string(),
                        public_dns: inst.public_dns_name().unwrap_or_default().to_string(),
                        tags: inst
                            .tags()
                            .iter()
                            .map(|t| {
                                (
                                    t.key().unwrap_or_default().to_string(),
                                    t.value().unwrap_or_default().to_string(),
                                )
                            })
                            .collect(),
                    },
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

    /// Switches between the Active Directory login and the user selected
    /// before it.
    pub fn toggle_ad_login(&mut self) {
        if !self.ssh_user.toggle_ad() {
            self.message =
                Some("No Active Directory account: add an [ad] section to the config".to_string());
        }
    }

    /// Describes how to connect to the selected instance with the current
    /// mode, user and key. The error is a message for the status line.
    pub fn connect_spec(&self) -> Result<ConnectSpec, String> {
        let item = self.selected_item.as_ref().ok_or("No instance selected")?;

        let target = match self.mode {
            ConnectMode::Ssm => Target::Ssm {
                instance_id: item.instance_id.clone(),
                region: Some(self.args.region.clone()).filter(|region| region != "None"),
                profile: Some(self.args.profile.clone()).filter(|profile| profile != "None"),
            },
            ConnectMode::Public | ConnectMode::Private => {
                let (host, kind) = match self.mode {
                    ConnectMode::Public => (&item.public_ipv4, "public"),
                    _ => (&item.private_ipv4, "private"),
                };
                // "-" is what the list shows for a missing address.
                if host.is_empty() || host == "-" {
                    return Err(format!("{} has no {kind} IP address", item.name));
                }

                let (user, auth) = match self.ssh_user.selected_ad() {
                    Some(ad) => (
                        ad.username.clone(),
                        SshAuth::Password {
                            autofill: ad.has_password,
                        },
                    ),
                    None => {
                        let key = self.ssh_keys.selected_key.clone().ok_or(
                            "No SSH key selected - add one to ~/.ssh or to [keys] in the config",
                        )?;
                        let user = self.ssh_user.selected_user.as_deref().unwrap_or("ec2-user");
                        (user.to_string(), SshAuth::Key(key))
                    }
                };
                Target::Ssh {
                    host: host.clone(),
                    user,
                    auth,
                }
            }
        };

        let mut spec = ConnectSpec::new(target);
        // A new window may not see $E2S_CONFIG, so name the file explicitly.
        if std::env::var_os("E2S_CONFIG").is_some() {
            spec.config_path = crate::utils::config::absolute_config_path()
                .and_then(|path| path.to_str().map(str::to_string));
        }
        Ok(spec)
    }
}
