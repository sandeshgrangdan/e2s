use crate::app::App;

use layout::Flex;
use ratatui::{
    prelude::*,
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Padding, Paragraph},
};

pub fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(layout::Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

pub fn render(_app: &mut App, f: &mut Frame) {
    let area = f.area();

    let area = popup_area(area, 70, 70);

    f.render_widget(ratatui::widgets::Clear, area); //this clears out the background

    let title_style = Style::default().bold().fg(Color::Gray);
    let key_style = Style::default().fg(Color::LightCyan);
    let desc_style = Style::default().fg(Color::Gray);

    let text = vec![
        Line::from(vec![Span::styled("[ General ]", title_style)]),
        Line::from(vec![
            Span::styled("        Ctrl-c: ", key_style),
            Span::styled("quit", desc_style),
        ]),
        Line::from(vec![
            Span::styled("   j k Down Up: ", key_style),
            Span::styled("scroll upward/downward", desc_style),
        ]),
        Line::from(vec![
            Span::styled("h l Left Right: ", key_style),
            Span::styled("Switch SSH Key", desc_style),
        ]),
        Line::from(vec![
            Span::styled("n m Left Right: ", key_style),
            Span::styled("Switch SSH user", desc_style),
        ]),
        Line::from(vec![
            Span::styled("           [/]: ", key_style),
            Span::styled("enable search mode", desc_style),
        ]),
        Line::from(vec![
            Span::styled("             s: ", key_style),
            Span::styled("SSH into selected", desc_style),
        ]),
        Line::from(vec![
            Span::styled("             p: ", key_style),
            Span::styled("Toggle Private/Public IP", desc_style),
        ]),
    ];

    f.render_widget(
        Paragraph::new(text)
            // .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .title(" + Help ")
                    .title_style(Style::default().bold().fg(Color::Gray))
                    .title_alignment(Alignment::Left)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Gray))
                    .padding(Padding::new(1, 1, 0, 1)),
            )
            .alignment(Alignment::Left),
        area,
    );

    // f.render_widget(block, area);
}
