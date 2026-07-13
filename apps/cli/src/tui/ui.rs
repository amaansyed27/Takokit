use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    Frame,
};

use super::app::{App, SpeakField, TranscribeField, TuiTab};

pub fn render(frame: &mut Frame<'_>, app: &App) {
    let page = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(12),
            Constraint::Length(8),
            Constraint::Length(2),
        ])
        .split(frame.area());
    render_header(frame, page[0], app);
    match app.tab {
        TuiTab::Models => render_models(frame, page[1], app),
        TuiTab::Speak => render_speak(frame, page[1], app),
        TuiTab::Transcribe => render_transcribe(frame, page[1], app),
        TuiTab::Runners => render_runners(frame, page[1], app),
        TuiTab::System => render_system(frame, page[1], app),
    }
    render_activity(frame, page[2], app);
    render_footer(frame, page[3], app);
    if app.show_help {
        render_help(frame);
    }
}

fn render_header(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let header = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(2)])
        .split(area);
    let state = app
        .running_label
        .as_ref()
        .map(|label| format!("  {} {label}", spinner(app.tick)))
        .unwrap_or_else(|| "  local voice runtime".to_string());
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "TAKOKIT",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(state, Style::default().add_modifier(Modifier::DIM)),
        ]))
        .block(Block::default().borders(Borders::BOTTOM)),
        header[0],
    );
    let labels = TuiTab::ALL
        .iter()
        .map(|tab| Line::from(tab.title()))
        .collect::<Vec<_>>();
    let selected = TuiTab::ALL
        .iter()
        .position(|tab| *tab == app.tab)
        .unwrap_or_default();
    frame.render_widget(
        Tabs::new(labels)
            .select(selected)
            .divider("  ")
            .highlight_style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            ),
        header[1],
    );
}

fn render_models(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let columns = split_columns(area);
    let items = app
        .models
        .iter()
        .map(|model| row_item(&model.title, &model.state))
        .collect::<Vec<_>>();
    render_list(frame, columns[0], " Models ", items, app.model_index);
    let detail = app
        .selected_model()
        .map(|model| model.detail.as_str())
        .unwrap_or("No models are available.");
    frame.render_widget(detail_panel(" Details ", detail), columns[1]);
}

fn render_runners(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let columns = split_columns(area);
    let items = app
        .runners
        .iter()
        .map(|runner| row_item(&runner.title, &runner.state))
        .collect::<Vec<_>>();
    render_list(frame, columns[0], " Runners ", items, app.runner_index);
    let detail = app
        .selected_runner()
        .map(|runner| runner.detail.as_str())
        .unwrap_or("No runners are available.");
    frame.render_widget(detail_panel(" Details ", detail), columns[1]);
}

fn render_system(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let columns = split_columns(area);
    let items = app
        .system
        .iter()
        .map(|row| row_item(row.title, row.state))
        .collect::<Vec<_>>();
    render_list(frame, columns[0], " System ", items, app.system_index);
    let detail = app
        .selected_system()
        .map(|row| row.detail)
        .unwrap_or("No system action is available.");
    frame.render_widget(detail_panel(" Details ", detail), columns[1]);
}

fn render_speak(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let form = centered_rect(82, 100, area);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(form);
    frame.render_widget(
        Paragraph::new("Choose a voice model, type text, and generate a local WAV."),
        rows[0],
    );
    let model = app.selected_speak_model();
    frame.render_widget(
        field(
            " Model · ↑/↓ change ",
            model
                .map(|model| format!("{}  ·  {}", model.title, model.state))
                .unwrap_or_else(|| "No TTS model available".to_string()),
            app.speak_field == SpeakField::Model,
        ),
        rows[1],
    );
    frame.render_widget(
        field(
            " Voice ",
            app.speak_voice.as_str(),
            app.speak_field == SpeakField::Voice,
        ),
        rows[2],
    );
    frame.render_widget(
        field(
            " Text ",
            if app.speak_text.is_empty() {
                "Type what Takokit should say…"
            } else {
                app.speak_text.as_str()
            },
            app.speak_field == SpeakField::Text,
        )
        .wrap(Wrap { trim: false }),
        rows[3],
    );
    let label = if model.is_some_and(|model| model.executable) {
        " Generate speech "
    } else {
        " Install selected model "
    };
    frame.render_widget(
        primary_button(label, app.speak_field == SpeakField::Primary),
        rows[4],
    );
    if app.speak_field == SpeakField::Voice {
        set_input_cursor(frame, rows[2], app.speak_voice_cursor);
    } else if app.speak_field == SpeakField::Text {
        set_input_cursor(frame, rows[3], app.speak_text_cursor);
    }
}

fn render_transcribe(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let form = centered_rect(82, 100, area);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(form);
    frame.render_widget(
        Paragraph::new("Choose a transcription model and enter a local audio file path."),
        rows[0],
    );
    let model = app.selected_transcribe_model();
    frame.render_widget(
        field(
            " Model · ↑/↓ change ",
            model
                .map(|model| format!("{}  ·  {}", model.title, model.state))
                .unwrap_or_else(|| "No STT model available".to_string()),
            app.transcribe_field == TranscribeField::Model,
        ),
        rows[1],
    );
    frame.render_widget(
        field(
            " Audio file ",
            if app.transcribe_audio.is_empty() {
                r#"C:\path\to\audio.wav"#
            } else {
                app.transcribe_audio.as_str()
            },
            app.transcribe_field == TranscribeField::Audio,
        ),
        rows[2],
    );
    let label = if model.is_some_and(|model| model.executable) {
        " Transcribe audio "
    } else {
        " Install selected model "
    };
    frame.render_widget(
        primary_button(label, app.transcribe_field == TranscribeField::Primary),
        rows[3],
    );
    if app.transcribe_field == TranscribeField::Audio {
        set_input_cursor(frame, rows[2], app.transcribe_audio_cursor);
    }
}

fn render_activity(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let title = if let Some(label) = &app.running_label {
        format!(" {} {label} ", spinner(app.tick))
    } else if let Some(label) = &app.last_label {
        format!(" Last result · {label} ")
    } else {
        " Activity ".to_string()
    };
    frame.render_widget(
        Paragraph::new(app.status.as_str())
            .scroll((app.output_scroll, 0))
            .wrap(Wrap { trim: false })
            .block(Block::default().title(title).borders(Borders::ALL)),
        area,
    );
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let text = match app.tab {
        TuiTab::Models => "↑/↓ select · Enter install/use · P install · X remove · ←/→ views · F1 help · Ctrl+C exit",
        TuiTab::Speak => "Tab next field · ↑/↓ choose model · type in Voice/Text · Enter continue/run · Ctrl+←/→ views",
        TuiTab::Transcribe => "Tab next field · ↑/↓ choose model · type audio path · Enter continue/run · Ctrl+←/→ views",
        TuiTab::Runners => "↑/↓ select · Enter add/install/check · P add · I install · D check · X remove · ←/→ views",
        TuiTab::System => "↑/↓ select · Enter run · ←/→ views · R refresh · F1 help · Ctrl+C exit",
    };
    frame.render_widget(
        Paragraph::new(text)
            .style(Style::default().add_modifier(Modifier::DIM))
            .alignment(Alignment::Left),
        area,
    );
}

fn render_list(
    frame: &mut Frame<'_>,
    area: Rect,
    title: &'static str,
    items: Vec<ListItem<'_>>,
    selected: usize,
) {
    let has_items = !items.is_empty();
    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_symbol("› ")
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );
    let mut state = ListState::default();
    if has_items {
        state.select(Some(selected));
    }
    frame.render_stateful_widget(list, area, &mut state);
}

fn row_item<'a>(title: &'a str, state: &'a str) -> ListItem<'a> {
    ListItem::new(Line::from(vec![
        Span::raw(format!("{title}  ")),
        Span::styled(state, Style::default().add_modifier(Modifier::DIM)),
    ]))
}

fn field<'a>(title: &'a str, value: impl Into<Text<'a>>, focused: bool) -> Paragraph<'a> {
    Paragraph::new(value).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(if focused {
                Style::default().fg(Color::White)
            } else {
                Style::default().add_modifier(Modifier::DIM)
            }),
    )
}

fn primary_button<'a>(label: &'a str, focused: bool) -> Paragraph<'a> {
    Paragraph::new(label)
        .alignment(Alignment::Center)
        .style(if focused {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            Style::default().add_modifier(Modifier::BOLD)
        })
        .block(Block::default().borders(Borders::ALL))
}

fn detail_panel<'a>(title: &'a str, detail: &'a str) -> Paragraph<'a> {
    Paragraph::new(detail)
        .wrap(Wrap { trim: false })
        .block(Block::default().title(title).borders(Borders::ALL))
}

fn split_columns(area: Rect) -> std::rc::Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(area)
}

fn set_input_cursor(frame: &mut Frame<'_>, area: Rect, cursor: usize) {
    let x = area
        .x
        .saturating_add(1 + cursor as u16)
        .min(area.right().saturating_sub(2));
    frame.set_cursor_position((x, area.y.saturating_add(1)));
}

fn render_help(frame: &mut Frame<'_>) {
    let area = centered_rect(78, 72, frame.area());
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(
            "Takokit TUI\n\nThis interface is task-based; you do not need to type CLI commands.\n\nModels\n  Select a model and press Enter. Takokit installs it when needed, or opens Speak/Transcribe when ready.\n\nSpeak\n  Tab between model, voice, text, and the Generate button. Type directly in the text fields.\n\nTranscribe\n  Tab between model, audio path, and the Transcribe button.\n\nRunners and System\n  Select an item and press Enter for the sensible default action.\n\nNavigation\n  Left/Right changes views on list screens. Ctrl+Left/Right works everywhere.\n  PageUp/PageDown scrolls activity. F1 or Enter closes this help. Ctrl+C exits.",
        )
        .wrap(Wrap { trim: false })
        .block(Block::default().title(" Help ").borders(Borders::ALL)),
        area,
    );
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
