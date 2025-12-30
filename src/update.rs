use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::app::input::user_input::{InputMode, UserInput};
use crate::app::{App, SelectedTab};
use crate::tui::Tui;

async fn open_editor(app: &mut App, tui: &mut Tui) {
    let _ = tui.init_ec2();

    if let Err(e) = app.ssh().await {
        eprintln!("Error SSH to the server: {}", e);
    }

    let _ = tui.exit_ec2();
}

pub async fn update(app: &mut App, key_event: KeyEvent, tui: &mut Tui) {
    match app.input_mode {
        InputMode::Normal => match key_event.code {
            KeyCode::Char('c') | KeyCode::Char('C')
                if key_event.modifiers == KeyModifiers::CONTROL =>
            {
                app.quit();
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                app.fetch_ec2_data().await;
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
                app.connect_mode.toggle();
            }
            KeyCode::Char('?') => {
                app.show_help = !app.show_help;
            }
            KeyCode::Char('s') | KeyCode::Enter => open_editor(app, tui).await,
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
