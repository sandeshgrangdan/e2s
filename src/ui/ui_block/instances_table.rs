use crate::app::App;
use ratatui::{
    prelude::*,
    style::Style,
    widgets::{Block, Borders, Cell, HighlightSpacing, Padding, Paragraph, Row, Table},
};

pub fn render(app: &mut App, f: &mut Frame, layout: Rect) {
    let title = if app.search.1.input.is_empty() {
        "EC2 Instances".to_string()
    } else {
        format!("[/] {}", app.search.1.input.clone())
    };

    let header_cells = [
        "Name",
        "Status",
        "Private IP",
        "Key Group",
        "AMI",
        "Public IP",
        "Instance ID",
    ]
    .iter()
    .map(|h| {
        Cell::from(*h).style(
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::BOLD),
        )
    });

    let header = Row::new(header_cells).height(1).bottom_margin(0);

    let rows = app.display_items.iter().map(|item| {
        let status_style = if item.status.to_lowercase() == "running" {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Red)
        };

        let cells = vec![
            Cell::from(item.name.clone()).style(Style::default().fg(Color::Gray)),
            Cell::from(item.status.clone()).style(status_style),
            Cell::from(item.private_ipv4.clone()).style(Style::default().fg(Color::Gray)),
            Cell::from(item.key_group.clone()).style(Style::default().fg(Color::Gray)),
            Cell::from(item.ami_id.clone()).style(Style::default().fg(Color::Gray)),
            Cell::from(item.public_ipv4.clone()).style(Style::default().fg(Color::Gray)),
            Cell::from(item.instance_id.clone()).style(Style::default().fg(Color::Gray)),
        ];

        Row::new(cells).height(1)
    });
    let bar = Span::styled(" >> ", Style::default().fg(Color::Cyan));

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
            .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
            .border_type(ratatui::widgets::BorderType::Plain),
    )
    .highlight_symbol(Text::from(vec![bar.into()]))
    // .highlight_symbol(">> ")
    .row_highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    );

    f.render_stateful_widget(table, layout, &mut app.state);

    // let header_style = Style::default()
    //     .fg(app.colors.header_fg)
    //     .bg(app.colors.header_bg);
    // let selected_row_style = Style::default()
    //     .add_modifier(Modifier::REVERSED)
    //     .fg(app.colors.selected_row_style_fg);
    // let selected_col_style = Style::default().fg(app.colors.selected_column_style_fg);
    // let selected_cell_style = Style::default()
    //     .add_modifier(Modifier::REVERSED)
    //     .fg(app.colors.selected_cell_style_fg);

    // let header_cells = ["Name", "Instance ID", "IP", "Status"]
    //     .iter()
    //     .map(|h| Cell::from(*h).style(Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD)));

    // let header = Row::new(header_cells)
    //     .height(1)
    //     .bottom_margin(0);
    // // let header = ["Name", "Instance Id", "IP", "Status"]
    // //     .into_iter()
    // //     .map(Cell::from)
    // //     .map(|h| Cell::from(*h).style(Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD)));
    // //     // .collect::<Row>()
    // //     // .style(header_style)
    // //     // .height(1);

    // let rows = app.items.iter().enumerate().map(|(i, data)| {
    //     let color = match i % 2 {
    //         0 => app.colors.normal_row_color,
    //         _ => app.colors.alt_row_color,
    //     };
    //     let item = data.ref_array();
    //     item.into_iter()
    //         .map(|content| Cell::from(Text::from(format!("\n{content}\n"))))
    //         .collect::<Row>()
    //         .style(Style::new().fg(app.colors.row_fg).bg(color))
    //         .height(3)
    // });
    // //let bar = " █>> ";
    // let bar = Span::styled(">>", Style::default().fg(Color::Cyan));
    // let t = Table::new(
    //     rows,
    //     [
    //         // + 1 is for padding.
    //         Constraint::Length(app.longest_item_lens.0 + 1),
    //         Constraint::Min(app.longest_item_lens.1),
    //         Constraint::Min(app.longest_item_lens.2),
    //         Constraint::Min(app.longest_item_lens.3),
    //     ],
    // )
    // .header(header)
    // .block(Block::default().borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM))
    // // The selection arrow ">> "
    // // .row_highlight_style(selected_row_style)
    // // .column_highlight_style(selected_col_style)
    // // .cell_highlight_style(selected_cell_style)
    // .highlight_symbol(Text::from(vec!["".into(), bar.into(), "".into()]))
    // // .bg(app.colors.buffer_bg)

    // // .highlight_spacing(HighlightSpacing::Always)
    // ;

    // f.render_stateful_widget(t, layout, &mut app.state);
}
