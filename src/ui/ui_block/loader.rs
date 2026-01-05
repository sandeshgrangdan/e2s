use ratatui::{
    prelude::*,
    style::{Color, Style},
    text::{Span, Text},
    widgets::{Block, BorderType, Borders, Padding, Paragraph, Wrap},
};

use crate::app::App;

pub fn render(app: &mut App, f: &mut Frame, layout: Rect) {
    let loading_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Reset))
        .padding(Padding::new(2, 1, 0, 0))
        .border_type(BorderType::Plain);

    let loading_text = vec![
        Span::styled(
            "⏳ Loading ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Fetching EC2 Data",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "...",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    let loading = Paragraph::new(Line::from(loading_text))
        .block(loading_block)
        .style(Style::default().bg(Color::Reset))
        .alignment(Alignment::Left);

    f.render_widget(loading, layout);
}
