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
        Span::raw("  local voice model runtime  ·  CLI / TUI / GUI unified"),
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
    let status = Paragraph::new(app.status.as_str())
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .title(" Command output ")
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
        Line::from(
            "Tab switch · j/k select · Enter run/inspect · p pull · i install · t test · x remove · / any CLI command · ? help",
        )
    };
    let command = Paragraph::new(content)
        .alignment(Alignment::Left)
        .block(Block::default().borders(Borders::TOP));
    frame.render_widget(command, area);

    if app.command_mode {
        let x = area
            .x
            .saturating_add(2 + app.command_input.chars().count() as u16);
        frame.set_cursor_position((x.min(area.right().saturating_sub(1)), area.y));
    }
}

fn render_help(frame: &mut Frame<'_>) {
    let area = centered_rect(88, 88, frame.area());
    frame.render_widget(Clear, area);
    let help = Paragraph::new(
        "Keyboard\n\nTab / h / l      switch section\nj / k / arrows   move selection\nEnter             inspect, run, or edit selected command\np                 pull selected model/runner contract\ni                 install selected runner runtime\nt                 edit model test command\nx                 remove selected model/runner\nd                 doctor\ns                 start managed daemon\ng                 open GUI\nr                 refresh shared state\n/                 run any public Takokit CLI command\nq / Esc / Ctrl+C  quit\n\nFull command palette\n\nThe palette accepts the same Clap grammar as direct CLI commands, including:\n\ndaemon start|stop|restart|status|logs\ngui · doctor [--json] · version · status · capabilities · ps\nmodels · runners · library models|runners · list models|runners|voices\npull <model> [--metadata-only] · show <model> · plan <model> [--json] · rm <model>\nspeak <text> [--model ...] [--voice ...]\nrun <model> <text> [--voice ...] OR run <model> --file <audio>\ntranscribe <audio> [--model ...]\nrunner pull|install|show|rm <runner> · runner doctor <runner> [--json]\nadapter list|install|doctor ...\nquickstart [--full] · deps doctor|bootstrap · samples create\ntest [model] [--suite fast|launch] [--run] [--file ...] [--json]\nclone <sample> --name ... · train <samples> --name ...\n\nQuoted text and Windows paths are supported. Foreground `serve` is intentionally blocked; use `daemon start`.\n\nPress ? or Esc to close this help.",
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
