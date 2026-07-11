use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    Frame,
};

use super::app::{App, TuiTab};

pub fn render(frame: &mut Frame<'_>, app: &App) {
    let page = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(10),
            Constraint::Length(6),
            Constraint::Length(3),
        ])
        .split(frame.area());

    render_header(frame, page[0], app);
    render_body(frame, page[1], app);
    render_status(frame, page[2], app);
    render_command_bar(frame, page[3], app);

    if app.show_help {
        render_help(frame);
    }
}

fn render_header(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let header = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(2)])
        .split(area);

    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "TAKOKIT",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  local voice model runtime"),
    ]))
    .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(title, header[0]);

    let labels = TuiTab::ALL
        .iter()
        .map(|tab| Line::from(tab.title()))
        .collect::<Vec<_>>();
    let selected = TuiTab::ALL
        .iter()
        .position(|tab| *tab == app.tab)
        .unwrap_or_default();
    let tabs = Tabs::new(labels)
        .select(selected)
        .divider("  ")
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        );
    frame.render_widget(tabs, header[1]);
}

fn render_body(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(area);

    match app.tab {
        TuiTab::Models => render_rows(frame, columns[0], " Models ", &app.models, app.model_index),
        TuiTab::Runners => render_rows(
            frame,
            columns[0],
            " Runners ",
            &app.runners,
            app.runner_index,
        ),
        TuiTab::System => {
            let navigation = Paragraph::new(
                "d  doctor\ns  start daemon\ng  open GUI\nr  refresh\n/  command palette\n?  help\nq  quit",
            )
            .block(Block::default().title(" Actions ").borders(Borders::ALL));
            frame.render_widget(navigation, columns[0]);
        }
    }

    let detail = Paragraph::new(app.selected_detail())
        .wrap(Wrap { trim: false })
        .block(Block::default().title(" Details ").borders(Borders::ALL));
    frame.render_widget(detail, columns[1]);
}

fn render_rows(
    frame: &mut Frame<'_>,
    area: Rect,
    title: &'static str,
    rows: &[super::app::TuiRow],
    selected: usize,
) {
    let items = rows
        .iter()
        .map(|row| {
            ListItem::new(Line::from(vec![
                Span::raw(format!("{}  ", row.title)),
                Span::styled(
                    row.state.clone(),
                    Style::default().add_modifier(Modifier::DIM),
                ),
            ]))
        })
        .collect::<Vec<_>>();
    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_symbol("› ")
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );
    let mut state = ListState::default();
    if !rows.is_empty() {
        state.select(Some(selected.min(rows.len() - 1)));
    }
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_status(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let status = Paragraph::new(app.status.as_str())
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .title(" Last action ")
                .borders(Borders::ALL),
        );
    frame.render_widget(status, area);
}

fn render_command_bar(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let content = if app.command_mode {
        Line::from(vec![
            Span::styled("/ ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(app.command_input.as_str()),
        ])
    } else {
        Line::from("Tab switch  ·  j/k select  ·  Enter inspect  ·  p pull  ·  i install runner  ·  / command  ·  ? help")
    };
    let command = Paragraph::new(content)
        .alignment(Alignment::Left)
        .block(Block::default().borders(Borders::TOP));
    frame.render_widget(command, area);

    if app.command_mode {
        let x = area
            .x
            .saturating_add(2 + app.command_input.chars().count() as u16);
        let y = area.y;
        frame.set_cursor_position((x.min(area.right().saturating_sub(1)), y));
    }
}

fn render_help(frame: &mut Frame<'_>) {
    let area = centered_rect(72, 78, frame.area());
    frame.render_widget(Clear, area);
    let help = Paragraph::new(
        "Keyboard\n\nTab / h / l      switch section\nj / k / arrows   move selection\nEnter             inspect selected item\np                 pull selected model\ni                 install selected runner\nd                 run doctor\ns                 start managed daemon\ng                 open GUI\nr                 refresh state\n/                 open command palette\nq / Esc / Ctrl+C  quit\n\nPalette commands\n\npull <model>\nplan <model>\nrunner install <runner>\nrunner show <runner>\ndoctor\ngui\nserver\nrefresh\nquit\n\nPress ? or Esc to close this help.",
    )
    .wrap(Wrap { trim: false })
    .block(Block::default().title(" Takokit TUI help ").borders(Borders::ALL));
    frame.render_widget(help, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
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
