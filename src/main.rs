// ANCHOR: imports_main
// ANCHOR: declare_mods
/// Application.
pub mod app;
/// Terminal events handler.
pub mod event;
/// Terminal user interface.
pub mod tui;
/// Widget renderer.
pub mod ui;
pub mod update;

/// Utility functions and helpers.
pub mod utils;

/// Application updater.
// ANCHOR_END: declare_mods
use app::connect::{self, askpass};
use app::App;
use clap::Parser;
use color_eyre::Result;
// use event::{Event, EventHandler};
use crossterm::event::{self as my_event};
use tui::Tui;
use update::update;
// use tokio::task;
// use tokio::sync::mpsc;
// ANCHOR_END: imports_main

// ANCHOR: main
fn main() -> Result<()> {
    // ssh started e2s to answer one of its questions: see `app::connect::askpass`.
    if askpass::requested() {
        std::process::exit(askpass::run());
    }

    // A new terminal window started e2s to run one connection: see `app::connect`.
    let mut args = std::env::args_os().skip(1);
    if args.next().is_some_and(|arg| arg == connect::SUBCOMMAND) {
        let tokens: Vec<String> = args.map(|arg| arg.to_string_lossy().into_owned()).collect();
        std::process::exit(connect::trampoline(tokens));
    }

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(run())
}

/// The terminal user interface.
async fn run() -> Result<()> {
    // use std::time::Instant;
    // Create an application.
    let mut app = App::new(app::Args::parse());
    app.set_ec2_client().await;

    // let start = Instant::now();
    app.fetch_ec2_data().await;
    // let duration = start.elapsed();
    // println!("fetch_ec2_data took: {:?}", duration);

    // Initialize the terminal user interface.
    let terminal = ratatui::init();

    // let events = EventHandler::new(10);
    let mut tui = Tui::new(terminal);

    // Start the main loop.
    while !&app.should_quit {
        app.set_ec2s();
        // Render the user interface.
        tui.draw(&mut app)?;

        // Handle events.

        if let Ok(my_event::Event::Key(key_event)) = my_event::read() {
            update(&mut app, key_event, &mut tui).await
        }
    }

    // Exit the user interface.
    ratatui::restore();
    // tui.exit()?;
    Ok(())
}
// ANCHOR_END: main
