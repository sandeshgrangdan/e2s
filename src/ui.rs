use ratatui::{layout::Layout, prelude::*};

mod ui_block;
use crate::app::App;

pub fn render(app: &mut App, f: &mut Frame) {
    let area = f.area();
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let top_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Constraint::Percentage(100)])
        .split(main_layout[0]);

    let main_app_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if app.search.0 {
            vec![Constraint::Length(3), Constraint::Min(20)]
        } else {
            vec![Constraint::Min(20)]
        })
        .split(main_layout[1]);

    let table_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Constraint::Percentage(100)])
        .split(if app.search.0 {
            main_app_layout[1]
        } else {
            main_app_layout[0]
        });

    ui_block::header::render(app, f, top_layout[0]);

    if app.display_items.is_empty() {
        ui_block::welcome::render(app, f, table_layout[0]);
    } else {
        ui_block::instances_table::render(app, f, table_layout[0]);
    }
    ui_block::footer::render(app, f, main_layout[2]);

    if app.search.0 {
        ui_block::search::render_search(app, f, main_app_layout[0]);
    }

    if app.show_help {
        ui_block::help::render(app, f);
    }
}
