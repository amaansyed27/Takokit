use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{
    app::{App, TuiAction, TuiTab},
    command,
};

impl App {
    pub(super) fn handle_key(&mut self, key: KeyEvent) -> Option<TuiAction> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Some(TuiAction::Quit);
        }

        if self.show_help {
            if matches!(key.code, KeyCode::Esc | KeyCode::F(1)) {
                self.show_help = false;
            }
            return None;
        }

        if self.command_mode {
            return self.handle_command_key(key);
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return self.handle_shortcut(key.code);
        }

        match key.code {
            KeyCode::Esc => Some(TuiAction::Quit),
            KeyCode::F(1) => {
                self.show_help = true;
                None
            }
            KeyCode::Char('/') => {
                self.open_template(String::new());
                None
            }
            KeyCode::Tab | KeyCode::Right => {
                self.tab = self.tab.next();
                None
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.tab = self.tab.previous();
                None
            }
            KeyCode::Up => {
                self.move_selection(-1);
                None
            }
            KeyCode::Down => {
                self.move_selection(1);
                None
            }
            KeyCode::PageUp => {
                self.output_scroll = self.output_scroll.saturating_sub(5);
                None
            }
            KeyCode::PageDown => {
                self.output_scroll = self.output_scroll.saturating_add(5);
                None
            }
            KeyCode::Enter => {
                self.prepare_selected_command();
                None
            }
            KeyCode::Char(character)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.open_template(character.to_string());
                None
            }
            _ => None,
        }
    }

    fn handle_shortcut(&mut self, code: KeyCode) -> Option<TuiAction> {
        match code {
            KeyCode::Char('p') if self.tab == TuiTab::Models => self.selected_cli(&["pull"]),
            KeyCode::Char('p') if self.tab == TuiTab::Runners => {
                self.selected_cli(&["runner", "pull"])
            }
            KeyCode::Char('i') if self.tab == TuiTab::Runners => {
                self.selected_cli(&["runner", "install"])
            }
            KeyCode::Char('x') if self.tab == TuiTab::Models => self.selected_cli(&["rm"]),
            KeyCode::Char('x') if self.tab == TuiTab::Runners => {
                self.selected_cli(&["runner", "rm"])
            }
            KeyCode::Char('t') if self.tab == TuiTab::Models => {
                if let Some(row) = self.selected_row() {
                    self.open_template(format!("test {} --run", row.id));
                }
                None
            }
            KeyCode::Char('d') => Some(TuiAction::RunCli(vec!["doctor".into()])),
            KeyCode::Char('g') => Some(TuiAction::RunCli(vec!["gui".into()])),
            KeyCode::Char('s') => Some(TuiAction::RunCli(vec!["daemon".into(), "start".into()])),
            KeyCode::Char('r') => Some(TuiAction::Refresh),
            _ => None,
        }
    }

    fn handle_command_key(&mut self, key: KeyEvent) -> Option<TuiAction> {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('a') => self.command_cursor = 0,
                KeyCode::Char('e') => self.command_cursor = self.command_len(),
                KeyCode::Char('u') => {
                    self.command_input.clear();
                    self.command_cursor = 0;
                    self.history_index = None;
                }
                _ => {}
            }
            return None;
        }

        match key.code {
            KeyCode::Esc => {
                self.command_mode = false;
                self.command_input.clear();
                self.command_cursor = 0;
                self.history_index = None;
            }
            KeyCode::Enter => return self.submit_command(),
            KeyCode::Left => self.command_cursor = self.command_cursor.saturating_sub(1),
            KeyCode::Right => {
                self.command_cursor = (self.command_cursor + 1).min(self.command_len())
            }
            KeyCode::Home => self.command_cursor = 0,
            KeyCode::End => self.command_cursor = self.command_len(),
            KeyCode::Backspace => self.delete_backward(),
            KeyCode::Delete => self.delete_forward(),
            KeyCode::Up => self.previous_history(),
            KeyCode::Down => self.next_history(),
            KeyCode::Char(character)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.insert_character(character)
            }
            _ => {}
        }
        None
    }

    fn submit_command(&mut self) -> Option<TuiAction> {
        if self.running_command.is_some() {
            self.set_status("A command is already running. You can keep editing this command and run it when the current job finishes.");
            return None;
        }

        match command::parse(&self.command_input) {
            Ok(action) => {
                let submitted = self.command_input.trim().to_string();
                if !submitted.is_empty()
                    && self.command_history.last().map(String::as_str) != Some(submitted.as_str())
                {
                    self.command_history.push(submitted);
                }
                self.command_input.clear();
                self.command_cursor = 0;
                self.command_mode = false;
                self.history_index = None;
                Some(action)
            }
            Err(error) => {
                self.set_status(error);
                None
            }
        }
    }

    fn prepare_selected_command(&mut self) {
        let Some(row) = self.selected_row().cloned() else {
            self.set_status("Nothing is selected in this section.");
            return;
        };
        let template = row
            .template
            .or_else(|| row.command.map(|args| command::format_args(&args)));
        match template {
            Some(value) => {
                self.open_template(value);
                self.set_status("Command loaded. Edit it if needed, then press Enter to run.");
            }
            None => self.set_status("The selected item has no command attached."),
        }
    }

    fn selected_cli(&self, command: &[&str]) -> Option<TuiAction> {
        let id = self.selected_row()?.id.clone();
        let mut args = command
            .iter()
            .map(|part| (*part).to_string())
            .collect::<Vec<_>>();
        args.push(id);
        Some(TuiAction::RunCli(args))
    }

    fn move_selection(&mut self, delta: isize) {
        let len = self.selected_rows().len();
        match self.tab {
            TuiTab::Models => self.model_index = shifted_index(self.model_index, len, delta),
            TuiTab::Runners => self.runner_index = shifted_index(self.runner_index, len, delta),
            TuiTab::Operations => {
                self.operation_index = shifted_index(self.operation_index, len, delta)
            }
            TuiTab::System => self.system_index = shifted_index(self.system_index, len, delta),
        }
    }

    fn open_template(&mut self, template: String) {
        self.command_cursor = template.chars().count();
        self.command_input = template;
        self.command_mode = true;
        self.history_index = None;
    }

    fn command_len(&self) -> usize {
        self.command_input.chars().count()
    }

    fn insert_character(&mut self, character: char) {
        let mut characters = self.command_input.chars().collect::<Vec<_>>();
        characters.insert(self.command_cursor.min(characters.len()), character);
        self.command_input = characters.into_iter().collect();
        self.command_cursor += 1;
        self.history_index = None;
    }

    fn delete_backward(&mut self) {
        if self.command_cursor == 0 {
            return;
        }
        let mut characters = self.command_input.chars().collect::<Vec<_>>();
        self.command_cursor -= 1;
        characters.remove(self.command_cursor);
        self.command_input = characters.into_iter().collect();
        self.history_index = None;
    }

    fn delete_forward(&mut self) {
        let mut characters = self.command_input.chars().collect::<Vec<_>>();
        if self.command_cursor < characters.len() {
            characters.remove(self.command_cursor);
            self.command_input = characters.into_iter().collect();
            self.history_index = None;
        }
    }

    fn previous_history(&mut self) {
        if self.command_history.is_empty() {
            return;
        }
        let index = self
            .history_index
            .unwrap_or(self.command_history.len())
            .saturating_sub(1);
        self.history_index = Some(index);
        self.command_input = self.command_history[index].clone();
        self.command_cursor = self.command_len();
    }

    fn next_history(&mut self) {
        let Some(index) = self.history_index else {
            return;
        };
        if index + 1 < self.command_history.len() {
            let next = index + 1;
            self.history_index = Some(next);
            self.command_input = self.command_history[next].clone();
        } else {
            self.history_index = None;
            self.command_input.clear();
        }
        self.command_cursor = self.command_len();
    }
}

fn shifted_index(current: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }
    (current as isize + delta).rem_euclid(len as isize) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_wraps_in_both_directions() {
        assert_eq!(shifted_index(0, 3, -1), 2);
        assert_eq!(shifted_index(2, 3, 1), 0);
        assert_eq!(shifted_index(0, 0, 1), 0);
    }
}
