use ratatui::{
    prelude::*,
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Padding, Paragraph, Wrap},
};

use crate::app::{App, aws::ec2::ConnectMode};

pub fn render(app: &mut App, f: &mut Frame, layout: Rect) {
    let title_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Reset))
        .padding(Padding::new(2, 1, 0, 0))
        .border_type(BorderType::Plain);

    let ssh_from_private = match app.connect_mode {
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

    let title = Paragraph::new(Text::styled(
        format!(
            " e2s - EC2 SSH Selector ({}) | Region: {} | SSH Key: {} | User: {}",
            ssh_from_private, app.args.region, selected_ssh_key, user
        ),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    ))
    .block(title_block)
    .style(Style::default().bg(Color::Reset))
    .alignment(Alignment::Left);

    f.render_widget(title, layout);
}
