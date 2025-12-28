use crate::app::App;
use ratatui::{
    prelude::*,
    style::{Color, Style},
    text::Text,
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

#[allow(dead_code)]
pub const BANNER: &str = r"
       ________          
  ____ \_____  \   ______
_/ __ \ /  ____/  /  ___/
\  ___//       \  \___ \ 
 \___  >_______ \/____  >
     \/        \/     \/ 
";

pub fn render(_app: &mut App, f: &mut Frame, layout: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Length(93)].as_ref())
        .margin(2)
        .split(layout);

    let welcome = Block::default()
        .title(Span::styled(
            "— No EC2 Instances Available —",
            Style::default().fg(Color::LightCyan),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::LightCyan))
        .border_type(BorderType::Rounded);

    f.render_widget(welcome, layout);

    let changelog = include_str!("../../../README.md").to_string();

    let clean_changelog = if cfg!(debug_assertions) {
        changelog
    } else {
        changelog.replace("\n## [Unreleased]\n", "")
    };

    let top_text_banner = Text::from(BANNER);

    let bottom_text_raw = format!(
        "{}{}",
        "\nPlease report any bugs or missing features to https://github.com/sandeshgrangdan/e2s\n\n",
        clean_changelog
      );
    let bottom_text = Text::from(bottom_text_raw.as_str());

    let top_text = Paragraph::new(top_text_banner)
        .style(Style::default().fg(Color::LightCyan))
        .block(Block::default());
    f.render_widget(top_text, chunks[0]);

    let bottom_text = Paragraph::new(bottom_text)
        .style(Style::default().fg(Color::LightCyan))
        .block(Block::default())
        .wrap(Wrap { trim: false })
        .scroll((0, 0));
    f.render_widget(bottom_text, chunks[1]);
}
