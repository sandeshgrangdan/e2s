use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::app::connect::{self, launcher};
use crate::app::input::user_input::InputMode;
use crate::app::App;
use crate::tui::Tui;

/// Connects to the selected instance: in a new terminal window if one is
/// configured, otherwise in this terminal with the TUI suspended.
async fn handle_ssh(app: &mut App, tui: &mut Tui) {
    let spec = match app.connect_spec() {
        Ok(spec) => spec,
        Err(message) => {
            app.message = Some(message);
            return;
        }
    };

    if let Some(emulator) = &app.terminal.terminal.emulator {
        if let Err(e) = launcher::open(emulator, &spec) {
            app.message = Some(format!("Cannot open a new terminal window: {e}"));
        }
    } else {
        let _ = tui.init_ec2_ssh();

        if let Err(e) = connect::run(&spec) {
            app.message = Some(format!("Cannot start the session: {e}"));
        }

        let _ = tui.exit_ec2_ssh();
    }
}

pub async fn update(app: &mut App, key_event: KeyEvent, tui: &mut Tui) {
    // Windows reports a key twice: when it is pressed and when it is released.
    if key_event.kind == KeyEventKind::Release {
        return;
    }
    // A notice stays until the next key press.
    app.message = None;

    match app.input_mode {
        InputMode::Normal => match key_event.code {
            KeyCode::Char('c') | KeyCode::Char('C')
                if key_event.modifiers == KeyModifiers::CONTROL =>
            {
                app.quit();
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                if app.show_help {
                    app.show_help = !app.show_help
                }
                app.show_details = false;
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                app.show_details = !app.show_details;
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                app.loading = true;
                app.fetch_ec2_data().await;
                app.loading = false;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.ec2_next();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.ec2_previous();
            }
            KeyCode::Right | KeyCode::Char('l') => {
                app.ssh_keys.next();
            }
            KeyCode::Left | KeyCode::Char('h') => {
                app.ssh_keys.previous();
            }
            KeyCode::Char('/') => {
                app.toggle_search();
            }
            KeyCode::Char('n') => {
                app.ssh_user.next();
            }
            KeyCode::Char('m') => {
                app.ssh_user.previous();
            }
            KeyCode::Char('p') => {
                app.mode.toggle();
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                app.toggle_ad_login();
            }
            KeyCode::Char('?') => {
                app.show_help = !app.show_help;
            }
            KeyCode::Char('s') | KeyCode::Enter => handle_ssh(app, tui).await,
            _ => {}
        },
        InputMode::Editing if key_event.kind == KeyEventKind::Press && app.search.0 => {
            match key_event.code {
                KeyCode::Enter => app.toggle_search(),
                KeyCode::Char(to_insert) => {
                    app.search.1.enter_char(to_insert);
                }
                KeyCode::Backspace => {
                    app.search.1.delete_char();
                }
                KeyCode::Left => {
                    app.search.1.move_cursor_left();
                }
                KeyCode::Right => {
                    app.search.1.move_cursor_right();
                }
                KeyCode::Down => {
                    app.ec2_next();
                }
                KeyCode::Up => {
                    app.ec2_previous();
                }
                KeyCode::Esc => {
                    app.toggle_search();
                }
                _ => {}
            }
        }
        InputMode::Editing => {}
    }
}
