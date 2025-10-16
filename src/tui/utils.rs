pub mod cursor {
    pub fn move_cursor_left(input: &String, char_index: &mut usize) {
        let cursor_moved_left = char_index.saturating_sub(1);
        *char_index = clamp_cursor(input, cursor_moved_left);
    }

    pub fn move_cursor_right(input: &String, char_index: &mut usize) {
        let cursor_moved_right = char_index.saturating_add(1);
        *char_index = clamp_cursor(input, cursor_moved_right);
    }

    pub fn enter_char(input: &mut String, char_index: &mut usize, new_char: char) {
        let index = byte_index(input, *char_index);
        input.insert(index, new_char);
        move_cursor_right(input, char_index);
    }

    /// Returns the byte index based on the character position.
    ///
    /// Since each character in a string can be contain multiple bytes, it's necessary to calculate
    /// the byte index based on the index of the character.
    pub fn byte_index(input: &String, character_index: usize) -> usize {
        input
            .char_indices()
            .map(|(i, _)| i)
            .nth(character_index)
            .unwrap_or(input.len())
    }

    pub fn delete_char(input: &mut String, char_index: &mut usize) {
        let is_not_cursor_leftmost = *char_index != 0;
        if is_not_cursor_leftmost {
            let current_index = *char_index;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = input.chars().take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = input.chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            *input = before_char_to_delete.chain(after_char_to_delete).collect();
            move_cursor_left(input, char_index);
        }
    }

    pub fn clamp_cursor(input: &String, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, input.chars().count())
    }

    pub fn reset_cursor(app: &mut crate::tui::App) {
        app.character_index = 0;
    }
}
