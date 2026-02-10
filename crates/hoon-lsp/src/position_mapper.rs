use ropey::Rope;
use tower_lsp::lsp_types::{Position, Range};

pub fn range_from_byte_offsets(text: &str, start_byte: usize, end_byte: usize) -> Range {
    let start = byte_offset_to_position_utf16(text, start_byte);
    let end = byte_offset_to_position_utf16(text, end_byte);
    Range { start, end }
}

pub fn byte_offset_to_position_utf16(text: &str, byte_offset: usize) -> Position {
    let clamped = byte_offset.min(text.len());
    let mut line: u32 = 0;
    let mut col_utf16: u32 = 0;
    let mut seen = 0usize;

    for ch in text.chars() {
        if seen >= clamped {
            break;
        }
        let len = ch.len_utf8();
        if seen + len > clamped {
            break;
        }
        if ch == '\n' {
            line += 1;
            col_utf16 = 0;
        } else {
            col_utf16 += ch.len_utf16() as u32;
        }
        seen += len;
    }

    Position {
        line,
        character: col_utf16,
    }
}

pub fn position_utf16_to_char_idx(rope: &Rope, position: Position) -> usize {
    let line = usize::try_from(position.line).unwrap_or(usize::MAX);
    if line >= rope.len_lines() {
        return rope.len_chars();
    }

    let line_start = rope.line_to_char(line);
    let line_slice = rope.line(line);
    let target_utf16 = usize::try_from(position.character).unwrap_or(usize::MAX);

    let mut utf16_col = 0usize;
    let mut chars_used = 0usize;
    for ch in line_slice.chars() {
        if utf16_col >= target_utf16 {
            break;
        }
        let w = ch.len_utf16();
        if utf16_col + w > target_utf16 {
            break;
        }
        utf16_col += w;
        chars_used += 1;
    }

    line_start + chars_used
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_offsets_ascii() {
        let text = "abc\ndef";
        let pos = byte_offset_to_position_utf16(text, 5);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 1);
    }

    #[test]
    fn byte_offsets_unicode_utf16_width() {
        let text = "a😀b\n";
        let pos_after_emoji = byte_offset_to_position_utf16(text, "a😀".len());
        assert_eq!(pos_after_emoji.line, 0);
        assert_eq!(pos_after_emoji.character, 3);
    }

    #[test]
    fn utf16_position_to_char_index() {
        let rope = Rope::from_str("a😀b\n");
        let idx = position_utf16_to_char_idx(
            &rope,
            Position {
                line: 0,
                character: 3,
            },
        );
        assert_eq!(idx, 2);
    }
}
