use std::{io, process::Command};

use aws_config::{
    meta::region::RegionProviderChain, profile::ProfileFileRegionProvider, BehaviorVersion, Region,
    SdkConfig,
};
use fakeit::internet;
use itertools::Itertools;
use rand::{seq::SliceRandom, Rng};

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
        }
    }

    pub async fn ssh(&mut self) -> io::Result<()> {
        if let Some(selected) = self.state.selected() {
            if let Some(item) = self.display_items.get(selected) {
                let key_path = match &self.ssh_keys.selected_key {
                    Some(key) => key,
                    None => {
                        println!("No SSH key selected.");
                        return Ok(());
                    }
                };

                let user = self.ssh_user.selected_user.as_deref().unwrap_or("ec2-user");
                let ip = if self.private {
                    &item.private_ipv4
                } else {
                    &item.public_ipv4
                };

                let status = Command::new("ssh")
                    .arg("-i")
                    .arg(key_path)
                    .arg(format!("{}@{}", user, ip))
                    .status()?;

                if !status.success() {
                    eprintln!("Failed to launch SSH session.");
                }
            }
        }
        Ok(())
    }
}
