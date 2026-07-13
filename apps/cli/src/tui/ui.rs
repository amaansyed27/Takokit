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
            Constraint::Length(9),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(frame.area());

    render_header(frame, page[0], app);
    render_body(frame, page[1], app);
    render_status(frame, page[2], app);
    render_command_bar(frame, page[3], app);
    render_footer(frame, page[4], app);

    if app.show_help {
        render_help(frame);
    }
}

fn render_header(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let header = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(2)])
        .split(area);

    let state = if let Some(command) = &app.running_command {
        format!("  {} running: takokit {command}", spinner(app.tick))
    } else if app.command_mode {
        "  INSERT · Enter runs · Esc cancels".to_string()
    } else {
        "  NAVIGATE · type anytime".to_string()
    };
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "TAKOKIT",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  local voice runtime"),
        Span::styled(state, Style::default().add_modifier(Modifier::DIM)),
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

    let title = match app.tab {
        TuiTab::Models => " Models ",
        TuiTab::Runners => " Runners ",
        TuiTab::Operations => " Operations ",
        TuiTab::System => " System ",
    };
    render_rows(
        frame,
        columns[0],
        title,
        app.selected_rows(),
        app.selected_index(),
    );

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
    let title = if let Some(command) = &app.running_command {
        format!(" {} Running takokit {command} ", spinner(app.tick))
    } else if let Some(command) = &app.last_command {
        format!(" Output · takokit {command} ")
    } else {
        " Output ".to_string()
    };
    let status = Paragraph::new(app.status.as_str())
        .scroll((app.output_scroll, 0))
        .wrap(Wrap { trim: false })
        .block(Block::default().title(title).borders(Borders::ALL));
    frame.render_widget(status, area);
}

fn render_command_bar(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let (content, title, style) = if app.command_mode {
        (
            Line::from(vec![
                Span::styled("> ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(app.command_input.as_str()),
            ]),
            " Command · Enter run · Esc cancel · ↑/↓ history ",
            Style::default().fg(Color::White),
        )
    } else {
        (
            Line::from("Type a command, press /, or select an item and press Enter…"),
            " Command bar ",
            Style::default().add_modifier(Modifier::DIM),
        )
    };
    let command = Paragraph::new(content)
        .style(style)
        .alignment(Alignment::Left)
        .block(Block::default().title(title).borders(Borders::ALL));
    frame.render_widget(command, area);

    if app.command_mode {
        let x = area
            .x
            .saturating_add(3 + app.command_cursor as u16)
            .min(area.right().saturating_sub(2));
        frame.set_cursor_position((x, area.y.saturating_add(1)));
    }
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let text = if app.command_mode {
        "←/→ edit · Home/End · Ctrl+U clear · Enter run · Esc cancel"
    } else {
        "Tab switch · ↑/↓ select · Enter prepare · type command · Ctrl+P pull · F1 help · Esc exit"
    };
    frame.render_widget(
        Paragraph::new(text)
            .style(Style::default().add_modifier(Modifier::DIM))
            .alignment(Alignment::Left),
        area,
    );
}

fn render_help(frame: &mut Frame<'_>) {
    let area = centered_rect(88, 88, frame.area());
    frame.render_widget(Clear, area);
    let help = Paragraph::new(
        "Interaction\n\nType any character  start entering a Takokit command\n/                   open an empty command bar\nEnter on a row      load its command into the bar\nEnter in command    validate and run it\nEsc in command      cancel command editing\n↑ / ↓ in command    browse command history\n← / → Home / End    edit at the cursor\nPageUp / PageDown   scroll command output\nTab / Shift+Tab     switch section\n↑ / ↓               move selection\nF1                  open or close this help\nEsc / Ctrl+C        exit from navigation mode\n\nDirect shortcuts\n\nCtrl+P  pull selected model or runner contract\nCtrl+I  install selected runner runtime\nCtrl+T  prepare a model test\nCtrl+X  remove selected model or runner\nCtrl+D  doctor\nCtrl+S  start managed daemon\nCtrl+G  open GUI\nCtrl+R  refresh shared state\n\nCommand bar\n\nThe command bar accepts the same Clap grammar as direct CLI commands. You may type either `plan whisper-tiny` or `takokit plan whisper-tiny`. Quoted text and Windows paths are supported. Foreground `serve` is blocked; use `daemon start`.\n\nCommands run in a background worker. The TUI remains visible and shows running state, captured stdout/stderr, errors, and completion timing.",
    )
    .wrap(Wrap { trim: false })
    .block(Block::default().title(" Takokit TUI help ").borders(Borders::ALL));
    frame.render_widget(help, area);
}

fn spinner(tick: u64) -> char {
    ['|', '/', '-', '\\'][(tick as usize) % 4]
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
