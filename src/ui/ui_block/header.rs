use ratatui::{
    prelude::*,
    style::{Color, Style},
    text::{Span, Text},
    widgets::{Block, BorderType, Borders, Padding, Paragraph, Wrap},
};

use crate::app::{aws::ec2::ConnectMode, App};

pub fn render(app: &mut App, f: &mut Frame, layout: Rect) {
    let title_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Reset))
        .padding(Padding::new(2, 1, 0, 0))
        .border_type(BorderType::Plain);

    let ssh_from_private = match app.mode {
        ConnectMode::Private => "Private",
        ConnectMode::Public => "Public",
        ConnectMode::Ssm => "SSM",
    };

    let selected_ssh_key = if Some(app.ssh_keys.selected_key.clone().unwrap_or_default()) != None {
        app.ssh_keys.selected_key.clone().unwrap_or_default()
    } else {
        "No SSH Key Selected".to_string()
    };

    let user = app.ssh_user.selected_user.as_deref().unwrap_or("ec2-user");

    // With the Active Directory account selected the login uses its password,
    // not an SSH key.
    let (auth_label, auth_value) = match app.ssh_user.selected_ad() {
        Some(ad) if ad.has_password => (" | Auth: ", "AD password (saved)".to_string()),
        Some(_) => (" | Auth: ", "AD password (ask)".to_string()),
        None => (" | SSH Key: ", selected_ssh_key),
    };

    let mut title_text = vec![
        Span::styled(
            "e2s - EC2 SSH Selector (",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            ssh_from_private,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            ") | Region: ",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            &app.args.region,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            auth_label,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            auth_value,
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " | User: ",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            user,
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    let title = Paragraph::new(Line::from(title_text))
        .block(title_block)
        .style(Style::default().bg(Color::Reset))
        .alignment(Alignment::Left);

    f.render_widget(title, layout);
}
