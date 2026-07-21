use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

pub fn render_menu(
    frame: &mut Frame<'_>,
    area: Rect,
    title: &str,
    items: &[(&str, &str)],
    selected: usize,
) {
    let rows = items
        .iter()
        .enumerate()
        .map(|(index, (label, detail))| {
            ListItem::new(Text::from(vec![
                Line::from(vec![
                    Span::styled(
                        format!(" {} ", index + 1),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(*label, Style::default().add_modifier(Modifier::BOLD)),
                ]),
                Line::from(Span::styled(
                    format!("     {detail}"),
                    Style::default().add_modifier(Modifier::DIM),
                )),
            ]))
        })
        .collect::<Vec<_>>();
    render_list(frame, area, title, rows, selected);
}

pub fn render_rows(
    frame: &mut Frame<'_>,
    area: Rect,
    title: &str,
    rows: Vec<(String, String)>,
    selected: usize,
) {
    let items = rows
        .into_iter()
        .map(|(label, state)| {
            ListItem::new(Line::from(vec![
                Span::styled(label, Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("  "),
                Span::styled(state, Style::default().add_modifier(Modifier::DIM)),
            ]))
        })
        .collect::<Vec<_>>();
    render_list(frame, area, title, items, selected);
}

fn render_list(
    frame: &mut Frame<'_>,
    area: Rect,
    title: &str,
    items: Vec<ListItem<'_>>,
    selected: usize,
) {
    let has_items = !items.is_empty();
    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(" {title} "))
                .borders(Borders::ALL),
        )
        .highlight_symbol("› ")
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD),
        );
    let mut state = ListState::default();
    if has_items {
        state.select(Some(selected));
    }
    frame.render_stateful_widget(list, area, &mut state);
}

pub fn field<'a>(title: &'a str, value: impl Into<Text<'a>>, focused: bool) -> Paragraph<'a> {
    Paragraph::new(value).block(
        Block::default()
            .title(format!(" {title} "))
            .borders(Borders::ALL)
            .border_style(if focused {
                Style::default().fg(Color::White)
            } else {
                Style::default().add_modifier(Modifier::DIM)
            }),
    )
}

pub fn primary_button<'a>(label: &'a str, focused: bool) -> Paragraph<'a> {
    Paragraph::new(label)
        .alignment(Alignment::Center)
        .style(if focused {
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().add_modifier(Modifier::BOLD)
        })
        .block(Block::default().borders(Borders::ALL))
}

pub fn detail_panel<'a>(title: &'a str, detail: impl Into<Text<'a>>) -> Paragraph<'a> {
    Paragraph::new(detail).wrap(Wrap { trim: false }).block(
        Block::default()
            .title(format!(" {title} "))
            .borders(Borders::ALL),
    )
}

pub fn empty_state(frame: &mut Frame<'_>, area: Rect, title: &str, detail: &str) {
    let box_area = centered_rect(72, 34, area);
    frame.render_widget(
        Paragraph::new(Text::from(vec![
            Line::from(Span::styled(
                title,
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                detail,
                Style::default().add_modifier(Modifier::DIM),
            )),
        ]))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false })
        .block(Block::default().borders(Borders::ALL)),
        box_area,
    );
}

pub fn set_input_cursor(frame: &mut Frame<'_>, area: Rect, cursor: usize) {
    let x = area
        .x
        .saturating_add(1 + cursor as u16)
        .min(area.right().saturating_sub(2));
    frame.set_cursor_position((x, area.y.saturating_add(1)));
}

pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
