use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn edit_text(value: &mut String, cursor: &mut usize, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char(character)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            let mut characters = value.chars().collect::<Vec<_>>();
            characters.insert((*cursor).min(characters.len()), character);
            *value = characters.into_iter().collect();
            *cursor += 1;
            true
        }
        KeyCode::Left => {
            *cursor = cursor.saturating_sub(1);
            true
        }
        KeyCode::Right => {
            *cursor = (*cursor + 1).min(value.chars().count());
            true
        }
        KeyCode::Home => {
            *cursor = 0;
            true
        }
        KeyCode::End => {
            *cursor = value.chars().count();
            true
        }
        KeyCode::Backspace => {
            if *cursor > 0 {
                let mut characters = value.chars().collect::<Vec<_>>();
                *cursor -= 1;
                characters.remove(*cursor);
                *value = characters.into_iter().collect();
            }
            true
        }
        KeyCode::Delete => {
            let mut characters = value.chars().collect::<Vec<_>>();
            if *cursor < characters.len() {
                characters.remove(*cursor);
                *value = characters.into_iter().collect();
            }
            true
        }
        _ => false,
    }
}

pub fn shifted_index(current: usize, len: usize, delta: isize) -> usize {
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

    #[test]
    fn text_editor_inserts_and_deletes_at_cursor() {
        let mut value = "ac".to_string();
        let mut cursor = 1;
        assert!(edit_text(
            &mut value,
            &mut cursor,
            KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE)
        ));
        assert_eq!(value, "abc");
        assert!(edit_text(
            &mut value,
            &mut cursor,
            KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)
        ));
        assert_eq!(value, "ac");
    }
}
