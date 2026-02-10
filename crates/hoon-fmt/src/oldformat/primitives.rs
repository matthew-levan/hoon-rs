//! Shared formatting primitives built on top of `Doc` helpers.

use crate::doc::Doc;
use crate::format::mode::FormatMode;
use crate::format::{tall_form, wide_form};

/// Choose tall or wide rune layout based on the current formatting mode.
pub fn tall_or_wide(
    mode: FormatMode,
    rune: &str,
    tall_children: Vec<Doc>,
    wide_children: Vec<Doc>,
) -> Doc {
    match mode {
        FormatMode::Tall => tall_form(rune, tall_children),
        FormatMode::Wide => wide_form(rune, wide_children),
    }
}
