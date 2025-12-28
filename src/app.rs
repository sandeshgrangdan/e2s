use aws::ec2::{generate_fake_instances, Data};
use clap::Parser;
use rand::Rng;

use crate::app::{
    aws::ec2::Ec2Client,
    input::{ssh_keys::SshKeys, ssh_user::SshUsers},
};
use input::user_input::{
    InputMode::{self, Editing, Normal},
    UserInput,
};
use ratatui::widgets::TableState;

// Public mod
pub mod aws;
pub mod input;

#[derive(Parser, Debug, Clone)]
pub enum SelectedTab {
    List,
    Details,
}

// ANCHOR_END: action
/// AWS Systems Manager - Parameter Store TUI Client
#[derive(Parser, Debug, Default, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Name of your AWS profile.
    #[arg(short, long, default_value_t = String::from("None"))]
    pub profile: String,

    /// AWS Region.
    #[arg(short, long, default_value_t = String::from("None"))]
    pub region: String,
}
/// Application.
#[derive(Debug, Clone)]
pub struct App {
    pub should_quit: bool,
    pub search: (bool, UserInput),
    pub args: Args,
    pub show_help: bool,
    pub input_mode: InputMode,
    pub items: Vec<Data>,
    pub display_items: Vec<Data>,
    pub state: TableState,
    pub ec2_client: Ec2Client,
    pub ssh_keys: SshKeys,
    pub ssh_user: SshUsers,
    pub private: bool,
}

// ANCHOR: application_impl
impl App {
    /// Constructs a new instance of [`App`].
    pub fn new(args: Args) -> Self {
        // let data_vec = generate_fake_instances();
        Self {
            should_quit: false,
            search: (false, UserInput::new()),
            input_mode: InputMode::Normal,
            args,
            show_help: false,
            items: Vec::new(),
            // items: data_vec,
            display_items: Vec::new(),
            state: TableState::default().with_selected(0),
            ec2_client: Ec2Client::None,
            ssh_keys: SshKeys::load(),
            ssh_user: SshUsers::load(),
            private: true,
        }
    }

    /// Set should_quit to true to quit the application.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn toggle_search(&mut self) {
        if self.search.0 {
            self.input_mode = Normal;
        } else {
            self.input_mode = Editing;
        }
        self.search.0 = !self.search.0
    }

    fn generate_random_file_name(&self) -> String {
        let mut rng = rand::thread_rng();
        let random_string: String = (0..10)
            .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
            .collect();
        format!("/tmp/{}.txt", random_string) // Creating the file in /tmp directory
    }
}
// ANCHOR_END: application_impl

// ANCHOR: application_test
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn toggle_search() {
        let mut app = App::new(Args::parse());
        app.toggle_search();
        assert!(app.search.0);
    }

    // #[test]
    // fn increment_scrol() {
    //     let mut app = App::new(Args::parse());
    //     app.increment_scrol();
    //     assert_eq!(app.scroll, 1);
    // }

    // #[test]
    // fn decrement_scrol() {
    //     let mut app = App::new(Args::parse());
    //     app.decrement_scrol();
    //     assert_eq!(app.scroll, 0);
    // }
    // #[test]
    // fn clear_scrol() {
    //     let mut app = App::new(Args::parse());
    //     app.clear_scrol();
    //     assert_eq!(app.scroll, 0);
    // }

    // #[test]
    // fn get_selected_value() {
    //     let mut app = App::new(Args::parse());
    //     let value = app.get_selected_value();
    //     assert_eq!(value, "".to_string());
    // }

    // #[test]
    // fn get_selected_ps_data() {
    //     let mut app = App::new(Args::parse());
    //     let ps_data = app.get_selected_ps_data();

    //     let none_ps_data = SelectedPsMetadata::None;

    //     assert_eq!(ps_data, none_ps_data);
    // }
}
// ANCHOR_END: application_test
