use super::*;

pub(super) fn render_convert_value(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    target: ConvertField,
    label: &str,
    value: &str,
) {
    frame.render_widget(field(label, value, app.convert_state.field == target), area);
}

pub(super) fn render_intro(frame: &mut Frame<'_>, area: Rect, title: &str, detail: &str) {
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                title,
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(detail),
        ]),
        area,
    );
}
