mod forms;
mod home;
mod lists;
mod widgets;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::app::{App, TuiScreen};

pub fn render(frame: &mut Frame<'_>, app: &App) {
    let page = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(frame.area());

    render_header(frame, page[0], app);
    match app.screen {
        TuiScreen::Home => home::render_home(frame, page[1], app),
        TuiScreen::Speak => forms::render_speak(frame, page[1], app),
        TuiScreen::Transcribe => forms::render_transcribe(frame, page[1], app),
        TuiScreen::Clone => forms::render_clone(frame, page[1], app),
        TuiScreen::Manage => home::render_manage(frame, page[1], app),
        TuiScreen::Models => lists::render_models(frame, page[1], app),
        TuiScreen::Runners => lists::render_runners(frame, page[1], app),
        TuiScreen::System => lists::render_system(frame, page[1], app),
        TuiScreen::Sessions => lists::render_sessions(frame, page[1], app),
        TuiScreen::Activity => lists::render_activity(frame, page[1], app),
    }
    render_status(frame, page[2], app);
    render_footer(frame, page[3], app);

    if app.show_help {
        render_help(frame, app);
    }
}

fn render_header(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let session = app.active_session().to_string();
    let ready_models = app.models.iter().filter(|model| model.executable).count();
    let ready_runners = app.runners.iter().filter(|runner| runner.ready).count();
    frame.render_widget(
        Paragraph::new(Text::from(vec![
            Line::from(vec![
                Span::styled(
                    "TAKOKIT",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  /  {}", app.screen.title().to_uppercase()),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(Span::styled(
                format!(
                    "{ready_models} ready model{} · {ready_runners} ready runner{} · session {}",
                    if ready_models == 1 { "" } else { "s" },
                    if ready_runners == 1 { "" } else { "s" },
                    &session[..8]
                ),
                Style::default().add_modifier(Modifier::DIM),
            )),
        ]))
        .block(Block::default().borders(Borders::BOTTOM)),
        area,
    );
}

fn render_status(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let title = if let Some(label) = &app.running_label {
        format!(" {} {label} ", spinner(app.tick))
    } else if let Some(label) = &app.last_label {
        format!(" Last result · {label} ")
    } else {
        " Status ".to_string()
    };
    let mut lines = app.status.lines();
    let first = lines.next().unwrap_or_default();
    let second = lines.next();
    let more = lines.next().is_some();
    let body = match (second, more) {
        (Some(second), true) => format!("{first}\n{second}  … open Activity for full output"),
        (Some(second), false) => format!("{first}\n{second}"),
        (None, _) => first.to_string(),
    };
    frame.render_widget(
        Paragraph::new(body)
            .wrap(Wrap { trim: false })
            .block(Block::default().title(title).borders(Borders::TOP)),
        area,
    );
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let text = match app.screen {
        TuiScreen::Home => "↑/↓ choose · Enter open · 1–6 shortcut · R refresh · F1 help · Esc quit",
        TuiScreen::Speak => "Tab next field · ↑/↓ model · Enter continue · Ctrl+Enter run · Esc home",
        TuiScreen::Transcribe => {
            "Tab next field · ↑/↓ model · Enter continue · Ctrl+Enter run · Esc home"
        }
        TuiScreen::Clone => {
            "Tab next field · ↑/↓ model · Space consent · Ctrl+Enter run · Esc home"
        }
        TuiScreen::Manage => "↑/↓ choose · Enter open · 1–3 shortcut · R refresh · Esc home",
        TuiScreen::Models => "↑/↓ select · Enter use/repair · P repair · X remove · R refresh · Esc back",
        TuiScreen::Runners => {
            "↑/↓ select · Enter next action · D check · I install · X remove · Esc back"
        }
        TuiScreen::System => "↑/↓ select · Enter run · R refresh · Esc back",
        TuiScreen::Sessions => "↑/↓ select · Enter open · N new · R refresh · Esc home",
        TuiScreen::Activity => "↑/↓ or PgUp/PgDn scroll · Home/End jump · Esc home",
    };
    frame.render_widget(
        Paragraph::new(text)
            .style(Style::default().add_modifier(Modifier::DIM))
            .alignment(Alignment::Left),
        area,
    );
}

fn render_help(frame: &mut Frame<'_>, app: &App) {
    let area = widgets::centered_rect(76, 72, frame.area());
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(
            "Takokit TUI\n\nStart on Home and choose a task with ↑/↓ and Enter, or press its number.\n\nSpeak, Transcribe, Clone\n  Tab moves through fields. Arrow keys change the model. Ctrl+Enter runs the task.\n\nManage\n  Installed Models contains local models only. Runners and System hold runtime maintenance actions.\n\nSessions\n  Enter opens a session. N creates a new one.\n\nActivity\n  Shows the complete output from the latest task or error.\n\nNavigation\n  Esc goes back. Esc on Home exits. F1 closes this help. Ctrl+C always exits when no task is running.",
        )
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .title(format!(" Help · {} ", app.screen.title()))
                .borders(Borders::ALL),
        ),
        area,
    );
}

fn spinner(tick: u64) -> char {
    ['|', '/', '-', '\\'][(tick as usize) % 4]
}
