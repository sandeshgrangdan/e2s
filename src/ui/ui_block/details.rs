use crate::app::App;
use crate::ui::ui_block::help::popup_area;

use ratatui::{
    prelude::*,
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap},
};

/// Popup with everything EC2 reported about the selected instance.
pub fn render(app: &mut App, f: &mut Frame) {
    let area = popup_area(f.area(), 70, 80);
    f.render_widget(Clear, area);

    let section_style = Style::default().bold().fg(Color::Rgb(135, 206, 250));
    let label_style = Style::default().fg(Color::LightCyan);
    let value_style = Style::default().fg(Color::Gray);

    let section = |title: &str| Line::from(Span::styled(format!("[ {title} ]"), section_style));
    let row = |label: &str, value: &str| {
        let value = if value.is_empty() { "-" } else { value };
        Line::from(vec![
            Span::styled(format!("{label:>18}: "), label_style),
            Span::styled(value.to_string(), value_style),
        ])
    };

    let mut title = " + Instance Details ".to_string();
    let mut text = Vec::new();
    match &app.selected_item {
        None => text.push(Line::from("No instance selected")),
        Some(item) => {
            let d = &item.details;
            title = format!(" + {} ", item.name);

            text.push(section("Instance"));
            text.push(row("Name", &item.name));
            text.push(row("Instance ID", &item.instance_id));
            text.push(row("State", &item.status));
            text.push(row("Type", &d.instance_type));
            text.push(row("Architecture", &d.architecture));
            text.push(row("Platform", &d.platform));
            text.push(row("AMI", &item.ami_id));
            text.push(row("Key pair", &item.key_group));
            text.push(row("IAM profile", &d.iam_profile));
            text.push(row("Launched", &d.launch_time));
            text.push(Line::default());

            text.push(section("Network"));
            text.push(row("Availability zone", &d.availability_zone));
            text.push(row("VPC", &d.vpc_id));
            text.push(row("Subnet", &d.subnet_id));
            text.push(row("Private IP", &item.private_ipv4));
            text.push(row("Private DNS", &d.private_dns));
            text.push(row("Public IP", &item.public_ipv4));
            text.push(row("Public DNS", &d.public_dns));
            text.push(row("Security groups", &d.security_groups.join(", ")));
            text.push(Line::default());

            text.push(section("Tags"));
            if d.tags.is_empty() {
                text.push(row("", "-"));
            }
            for (key, value) in &d.tags {
                text.push(row(key, value));
            }
        }
    }

    f.render_widget(
        Paragraph::new(text).wrap(Wrap { trim: false }).block(
            Block::default()
                .title(title)
                .title_bottom(" d / Esc: close ")
                .title_style(Style::default().bold().fg(Color::Gray))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(100, 149, 237)))
                .padding(Padding::new(1, 1, 0, 0)),
        ),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::aws::ec2::{Data, Details};
    use crate::app::Args;
    use ratatui::{backend::TestBackend, Terminal};

    fn screen(app: &mut App) -> String {
        let mut terminal = Terminal::new(TestBackend::new(110, 40)).unwrap();
        terminal.draw(|f| render(app, f)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        buffer
            .content
            .chunks(110)
            .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn shows_what_ec2_reported_and_dashes_for_the_rest() {
        let mut app = App::new(Args::default());
        app.selected_item = Some(Data {
            name: "web-alpha".into(),
            instance_id: "i-0a11111111111111a".into(),
            ami_id: "ami-0123".into(),
            key_group: "dev-key".into(),
            public_ipv4: "-".into(),
            private_ipv4: "10.0.1.11".into(),
            status: "running".into(),
            details: Details {
                instance_type: "t3.medium".into(),
                iam_profile: "web-role".into(),
                security_groups: vec!["web (sg-01)".into(), "ssh (sg-02)".into()],
                tags: vec![("Env".into(), "prod".into())],
                ..Details::default()
            },
        });
        let screen = screen(&mut app);
        for expected in [
            "+ web-alpha",
            "Type: t3.medium",
            "IAM profile: web-role",
            "Security groups: web (sg-01), ssh (sg-02)",
            "Env: prod",
            "VPC: -",
            "d / Esc: close",
        ] {
            assert!(
                screen.contains(expected),
                "missing {expected:?} in:\n{screen}"
            );
        }
    }

    #[test]
    fn copes_with_no_selection() {
        let mut app = App::new(Args::default());
        app.selected_item = None;
        assert!(screen(&mut app).contains("No instance selected"));
    }
}
