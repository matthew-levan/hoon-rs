//! Formatting trait and implementations for Hoon AST.

pub mod atoms;
pub mod hoon;
pub mod runes;
pub mod spec;

use crate::config::FormatterConfig;
use crate::doc::Doc;

/// Trait for types that can be formatted to a Doc.
pub trait Format {
    /// Convert this value to a document representation.
    fn format(&self, config: &FormatterConfig) -> Doc;
}

/// Helper to format with a 2-space gap separator (tall form).
pub fn tall_form(rune: &str, children: Vec<Doc>) -> Doc {
    let mut parts = vec![Doc::text(rune)];
    for child in children {
        parts.push(Doc::gap());
        parts.push(child);
    }
    Doc::concat(parts)
}

/// Helper to format with parentheses and spaces (wide form).
pub fn wide_form(rune: &str, children: Vec<Doc>) -> Doc {
    let inner = Doc::join(Doc::text(" "), children);
    Doc::concat(vec![
        Doc::text(rune),
        Doc::text("("),
        inner,
        Doc::text(")"),
    ])
}

/// Create a tall/wide choice - tries wide first, falls back to tall.
pub fn tall_or_wide(rune: &str, children: Vec<Doc>) -> Doc {
    let wide = wide_form(rune, children.clone());
    let tall = tall_form(rune, children);
    Doc::group(Doc::flat_alt(tall, wide))
}

/// Format a 2-child rune with backstep (last child at same indent as rune).
/// Uses 2-space gaps, hardline before the backstep element.
pub fn rune2_backstep(rune: &str, first: Doc, second: Doc, indent: i32) -> Doc {
    Doc::concat(vec![
        Doc::text(rune),
        Doc::gap(),
        Doc::nest(indent, first),
        Doc::hardline(),
        second, // Last child at same level (backstep)
    ])
}

/// Format a 3-child rune with backstep.
/// Uses 2-space gaps, hardline before the backstep element.
pub fn rune3_backstep(rune: &str, first: Doc, second: Doc, third: Doc, indent: i32) -> Doc {
    Doc::concat(vec![
        Doc::text(rune),
        Doc::gap(),
        Doc::nest(indent, first),
        Doc::gap(),
        Doc::nest(indent, second),
        Doc::hardline(),
        third, // Last child at same level (backstep)
    ])
}

/// Format a 4-child rune with backstep.
/// Uses 2-space gaps, hardline before the backstep element.
pub fn rune4_backstep(
    rune: &str,
    first: Doc,
    second: Doc,
    third: Doc,
    fourth: Doc,
    indent: i32,
) -> Doc {
    Doc::concat(vec![
        Doc::text(rune),
        Doc::gap(),
        Doc::nest(indent, first),
        Doc::gap(),
        Doc::nest(indent, second),
        Doc::gap(),
        Doc::nest(indent, third),
        Doc::hardline(),
        fourth, // Last child at same level (backstep)
    ])
}

/// Format a variable-arity rune with == termination.
/// Uses 2-space gaps, breaks lines only when exceeding max_width.
pub fn rune_vararg(rune: &str, children: Vec<Doc>, indent: i32) -> Doc {
    let mut parts = vec![Doc::text(rune)];
    for child in children {
        parts.push(Doc::gap());
        parts.push(Doc::nest(indent, child));
    }
    parts.push(Doc::gap());
    parts.push(Doc::text("=="));
    Doc::concat(parts)
}

/// Format a list of items with == termination (tall form only).
pub fn terminated_list(children: Vec<Doc>, indent: i32) -> Doc {
    let mut parts = Vec::new();
    for child in children {
        parts.push(Doc::gap());
        parts.push(Doc::nest(indent, child));
    }
    parts.push(Doc::gap());
    parts.push(Doc::text("=="));
    Doc::concat(parts)
}
