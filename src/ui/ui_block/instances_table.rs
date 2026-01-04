use crate::app::App;
use ratatui::{
    prelude::*,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
};

pub fn render(app: &mut App, f: &mut Frame, layout: Rect) {
    let instance_name = app
        .selected_item
        .as_ref()
        .map(|item| item.name.clone())
        .unwrap_or_else(|| String::from("None"));

    let title = if app.search.1.input.is_empty() {
        Line::from(vec![
            Span::styled("╭─ ", Style::default().fg(Color::Rgb(100, 149, 237))),
            Span::styled(
                "EC2 Instances ",
                Style::default().fg(Color::Rgb(135, 206, 250)).bold(),
            ),
            Span::styled("│ ", Style::default().fg(Color::DarkGray)),
            Span::styled("Selected: ", Style::default().fg(Color::Gray)),
            Span::styled(
                instance_name,
                Style::default().fg(Color::Rgb(255, 182, 193)).bold(),
            ),
            Span::styled(" ", Style::default().fg(Color::Rgb(100, 149, 237))),
        ])
    } else {
        Line::from(vec![
            Span::styled("╭─ ", Style::default().fg(Color::Rgb(100, 149, 237))),
            Span::styled("🔍 ", Style::default().fg(Color::Yellow)),
            Span::styled(
                &app.search.1.input,
                Style::default().fg(Color::Rgb(144, 238, 144)).bold(),
            ),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            Span::styled("EC2: ", Style::default().fg(Color::Gray)),
            Span::styled(
                instance_name,
                Style::default().fg(Color::Rgb(255, 182, 193)).bold(),
            ),
            Span::styled(" ", Style::default().fg(Color::Rgb(100, 149, 237))),
        ])
    };

    let header_cells = [
        ("Name", Color::Rgb(173, 216, 230)),
        ("Status", Color::Rgb(255, 218, 185)),
        ("Private IP", Color::Rgb(221, 160, 221)),
        ("Key Group", Color::Rgb(152, 251, 152)),
        ("AMI", Color::Rgb(255, 222, 173)),
        ("Public IP", Color::Rgb(176, 196, 222)),
        ("Instance ID", Color::Rgb(255, 192, 203)),
    ]
    .iter()
    .map(|(name, color)| {
        Cell::from(*name).style(
            Style::default()
                .fg(*color)
                .bold()
                .add_modifier(Modifier::UNDERLINED),
        )
    });

    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = app.display_items.iter().enumerate().map(|(idx, item)| {
        let (status_style, status_icon) = match item.status.to_lowercase().as_str() {
            "running" => (Style::default().fg(Color::Rgb(144, 238, 144)).bold(), "●"),
            "stopped" => (Style::default().fg(Color::Rgb(255, 99, 71)).bold(), "■"),
            "terminated" => (Style::default().fg(Color::Rgb(128, 128, 128)), "✕"),
            "pending" => (Style::default().fg(Color::Rgb(255, 215, 0)).bold(), "◐"),
            "stopping" => (Style::default().fg(Color::Rgb(255, 165, 0)).bold(), "◎"),
            _ => (Style::default().fg(Color::White), "○"),
        };

        // Alternate row background for better readability
        let base_style = if idx % 2 == 0 {
            Style::default().fg(Color::Rgb(220, 220, 220))
        } else {
            Style::default().fg(Color::Rgb(200, 200, 200))
        };

        let cells = vec![
            Cell::from(format!(" {}", item.name)).style(base_style.fg(Color::Rgb(173, 216, 230))),
            Cell::from(format!("{} {}", status_icon, item.status)).style(status_style),
            Cell::from(item.private_ipv4.as_str()).style(base_style.fg(Color::Rgb(221, 160, 221))),
            Cell::from(item.key_group.as_str()).style(base_style.fg(Color::Rgb(152, 251, 152))),
            Cell::from(item.ami_id.as_str()).style(base_style.fg(Color::Rgb(255, 222, 173))),
            Cell::from(item.public_ipv4.as_str()).style(base_style.fg(Color::Rgb(176, 196, 222))),
            Cell::from(item.instance_id.as_str()).style(base_style.fg(Color::Rgb(255, 192, 203))),
        ];

        Row::new(cells).height(1)
    });

    let bar = Span::styled(
        " ▶ ",
        Style::default()
            .fg(Color::Rgb(100, 200, 255))
            .bold()
            .add_modifier(Modifier::RAPID_BLINK),
    );

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(24), // Name
            Constraint::Percentage(6),  // Status
            Constraint::Percentage(12), // Private IP
            Constraint::Percentage(14), // Key Group
            Constraint::Percentage(18), // AMI
            Constraint::Percentage(12), // Public IP
            Constraint::Percentage(14), // Instance ID
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(100, 149, 237)))
            .border_type(ratatui::widgets::BorderType::Rounded),
    )
    .highlight_symbol(Text::from(vec![bar.into()]))
    .row_highlight_style(
        Style::default()
            .bg(Color::Rgb(47, 79, 79))
            .fg(Color::Rgb(255, 255, 255))
            .bold()
            .add_modifier(Modifier::ITALIC),
    );

    f.render_stateful_widget(table, layout, &mut app.state);
}
