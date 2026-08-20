use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn insert_text(value: &mut String, cursor: &mut usize, text: &str) {
    let mut characters = value.chars().collect::<Vec<_>>();
    let insert_at = (*cursor).min(characters.len());
    let inserted = text.chars().collect::<Vec<_>>();
    let inserted_len = inserted.len();
    characters.splice(insert_at..insert_at, inserted);
    *value = characters.into_iter().collect();
    *cursor = insert_at + inserted_len;
}

pub fn edit_text(value: &mut String, cursor: &mut usize, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            value.clear();
            *cursor = 0;
            true
        }
        KeyCode::Char(character)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            let mut buffer = [0_u8; 4];
            insert_text(value, cursor, character.encode_utf8(&mut buffer));
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

    #[test]
    fn ctrl_u_clears_text_field() {
        let mut value = "C:\\very\\long\\path.wav".to_string();
        let mut cursor = value.chars().count();
        assert!(edit_text(
            &mut value,
            &mut cursor,
            KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL)
        ));
        assert!(value.is_empty());
        assert_eq!(cursor, 0);
    }

    #[test]
    fn pasted_multiline_unicode_text_is_inserted_atomically() {
        let mut value = "Start  end".to_string();
        let mut cursor = 6;
        insert_text(&mut value, &mut cursor, "hello\nनमस्ते ü");
        assert_eq!(value, "Start hello\nनमस्ते ü end");
        assert_eq!(cursor, 20);
    }
}
