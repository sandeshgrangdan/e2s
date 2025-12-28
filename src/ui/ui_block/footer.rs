use crate::app::App;
use ratatui::{
    prelude::*,
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Padding, Paragraph},
};

pub fn render(app: &mut App, f: &mut Frame, layout: Rect) {
    let footer_text = Line::from(vec![
        Span::styled("[s]", Style::default().fg(Color::Cyan)),
        Span::styled(" SSH into selected ", Style::default().fg(Color::Gray)),
        Span::styled("[p]", Style::default().fg(Color::Cyan)),
        Span::styled(
            " Toggle Private/Public IP ",
            Style::default().fg(Color::Gray),
        ),
        Span::styled("[/]", Style::default().fg(Color::Cyan)),
        Span::styled(" Filter ", Style::default().fg(Color::Gray)),
        Span::styled("[j/k]", Style::default().fg(Color::Cyan)),
        Span::styled(" Navigate ", Style::default().fg(Color::Gray)),
        Span::styled("[h/l]", Style::default().fg(Color::Cyan)),
        Span::styled(" Switch SSH Key ", Style::default().fg(Color::Gray)),
        Span::styled("[n/m]", Style::default().fg(Color::Cyan)),
        Span::styled(" Switch SSH user ", Style::default().fg(Color::Gray)),
        Span::styled("[Ctrl-c:]", Style::default().fg(Color::Cyan)),
        Span::styled(" Quit", Style::default().fg(Color::Gray)),
    ]);

    let footer = Paragraph::new(footer_text).alignment(Alignment::Left);

    f.render_widget(footer, layout);
}
