//! Formatting trait and implementations for Hoon AST.

pub mod atoms;
pub mod block;
pub mod hoon;
pub mod mode;
pub mod primitives;
pub mod spec;
pub mod symbols;

use crate::config::FormatterConfig;
use crate::doc::Doc;
use crate::format::symbols::{ACE, GAP};

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
    let inner = Doc::join(Doc::text(ACE), children);
    Doc::concat(vec![Doc::text(rune), Doc::text("("), inner, Doc::text(")")])
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunePrefix {
    None,
    LeadingHardline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuneBreak {
    BeforeFirst,
    AfterFirst,
    AfterSecond,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuneLineMode {
    FitOrBreak,
    AlwaysBreak,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuneStyle {
    pub prefix: RunePrefix,
    pub break_mode: RuneBreak,
    pub line_mode: RuneLineMode,
    pub nest_continuation: bool,
    pub strip_leading_hardline: bool,
}

impl RuneStyle {
    /// Backstep style for runes that may stay flat, but when they break,
    /// keep the rune and first child on the same line.
    ///
    /// Shape:
    /// - flat: `rune  first  second ...`
    /// - broken: `rune  first` then continuation on the next line, nested
    ///   by `indent`.
    ///
    /// Used for compact 2-child runes where the first child is part of the
    /// header and the second child is the natural continuation.
    pub fn backstep_after_first() -> Self {
        Self {
            prefix: RunePrefix::None,
            break_mode: RuneBreak::AfterFirst,
            line_mode: RuneLineMode::FitOrBreak,
            nest_continuation: true,
            strip_leading_hardline: false,
        }
    }

    /// Backstep style for runes that may stay flat, but when they break,
    /// break immediately after the rune token.
    ///
    /// Shape:
    /// - flat: `rune  first  second ...`
    /// - broken: `rune` then all children on the next line, nested by `indent`.
    ///
    /// Used for wider multi-child backstep forms where the whole payload is
    /// treated as a continuation block.
    pub fn backstep_before_first() -> Self {
        Self {
            prefix: RunePrefix::None,
            break_mode: RuneBreak::BeforeFirst,
            line_mode: RuneLineMode::FitOrBreak,
            nest_continuation: true,
            strip_leading_hardline: false,
        }
    }

    /// Leading style for runes that must start on a fresh line and always
    /// break after the first child.
    ///
    /// Shape:
    /// - always emits a leading hardline before the rune
    /// - `rune  first` on one line
    /// - continuation starts on the next line without extra nesting
    ///
    /// The continuation can have one leading hardline stripped to avoid
    /// double-blank-line artifacts when child docs already begin with a break.
    pub fn leading_after_first() -> Self {
        Self {
            prefix: RunePrefix::LeadingHardline,
            break_mode: RuneBreak::AfterFirst,
            line_mode: RuneLineMode::AlwaysBreak,
            nest_continuation: false,
            strip_leading_hardline: true,
        }
    }

    /// Leading style for runes that must start on a fresh line and always
    /// break before the first child.
    ///
    /// Shape:
    /// - always emits a leading hardline before the rune
    /// - rune token alone on its line
    /// - all children on the next line, nested by `indent`
    ///
    /// This is useful for rune families whose conventional tall form presents
    /// a full continuation block under the rune.
    pub fn leading_before_first() -> Self {
        Self {
            prefix: RunePrefix::LeadingHardline,
            break_mode: RuneBreak::BeforeFirst,
            line_mode: RuneLineMode::AlwaysBreak,
            nest_continuation: true,
            strip_leading_hardline: false,
        }
    }

    /// Leading style for 3+ child runes that must start on a fresh line and
    /// always break after the second child.
    ///
    /// Shape:
    /// - always emits a leading hardline before the rune
    /// - `rune  first  second` kept together as the header
    /// - remaining continuation on the next line without extra nesting
    ///
    /// This is used for forms like `=*` where the first two children belong
    /// to the header and only the body should move to the continuation line.
    pub fn leading_after_second() -> Self {
        Self {
            prefix: RunePrefix::LeadingHardline,
            break_mode: RuneBreak::AfterSecond,
            line_mode: RuneLineMode::AlwaysBreak,
            nest_continuation: false,
            strip_leading_hardline: true,
        }
    }

    /// Block style for runes that should always break after the first child
    /// without injecting an extra leading hardline before the rune.
    ///
    /// Shape:
    /// - `rune  first`
    /// - continuation on the next line, nested by `indent`
    ///
    /// This is useful for block-position runes where parent formatting already
    /// controls line starts (e.g. inside arm bodies).
    pub fn block_after_first() -> Self {
        Self {
            prefix: RunePrefix::None,
            break_mode: RuneBreak::AfterFirst,
            line_mode: RuneLineMode::AlwaysBreak,
            nest_continuation: true,
            strip_leading_hardline: true,
        }
    }
}

fn break_then_continuation(break_doc: Doc, continuation: Doc, indent: i32, nest: bool) -> Doc {
    if nest {
        Doc::nest(indent, Doc::concat(vec![break_doc, continuation]))
    } else {
        Doc::concat(vec![break_doc, continuation])
    }
}

fn with_prefix(prefix: RunePrefix, body: Doc) -> Doc {
    match prefix {
        RunePrefix::None => body,
        RunePrefix::LeadingHardline => Doc::concat(vec![Doc::hardline(), body]),
    }
}

fn strip_one_leading_hardline(doc: Doc) -> Doc {
    match doc {
        Doc::HardLine => Doc::nil(),
        Doc::Concat(mut parts) => {
            if matches!(parts.first(), Some(Doc::HardLine)) {
                parts.remove(0);
                Doc::concat(parts)
            } else {
                Doc::Concat(parts)
            }
        }
        other => other,
    }
}

/// Format a 2-child rune using a configurable line-breaking style.
pub fn rune2(rune: &str, first: Doc, second: Doc, indent: i32, style: RuneStyle) -> Doc {
    let flat = Doc::concat(vec![
        Doc::text(rune),
        Doc::text(GAP),
        first.clone(),
        Doc::text(GAP),
        second.clone(),
    ]);

    let continuation = if style.strip_leading_hardline {
        strip_one_leading_hardline(second)
    } else {
        second
    };

    let break_doc = match style.line_mode {
        RuneLineMode::FitOrBreak => Doc::line(),
        RuneLineMode::AlwaysBreak => Doc::hardline(),
    };

    let broken = match style.break_mode {
        RuneBreak::BeforeFirst => {
            let tail = Doc::concat(vec![first, Doc::text(GAP), continuation]);
            Doc::concat(vec![
                Doc::text(rune),
                break_then_continuation(break_doc.clone(), tail, indent, style.nest_continuation),
            ])
        }
        RuneBreak::AfterFirst => Doc::concat(vec![
            Doc::text(rune),
            Doc::text(GAP),
            first,
            break_then_continuation(break_doc, continuation, indent, style.nest_continuation),
        ]),
        RuneBreak::AfterSecond => Doc::concat(vec![
            Doc::text(rune),
            Doc::text(GAP),
            first,
            break_then_continuation(break_doc, continuation, indent, style.nest_continuation),
        ]),
    };

    let body = match style.line_mode {
        RuneLineMode::FitOrBreak => Doc::group(Doc::flat_alt(broken, flat)),
        RuneLineMode::AlwaysBreak => broken,
    };

    with_prefix(style.prefix, body)
}

/// Format a 3-child rune using a configurable line-breaking style.
pub fn rune3(
    rune: &str,
    first: Doc,
    second: Doc,
    third: Doc,
    indent: i32,
    style: RuneStyle,
) -> Doc {
    let flat = Doc::concat(vec![
        Doc::text(rune),
        Doc::text(GAP),
        first.clone(),
        Doc::text(GAP),
        second.clone(),
        Doc::text(GAP),
        third.clone(),
    ]);

    let break_doc = match style.line_mode {
        RuneLineMode::FitOrBreak => Doc::line(),
        RuneLineMode::AlwaysBreak => Doc::hardline(),
    };

    let broken = match style.break_mode {
        RuneBreak::BeforeFirst => {
            let start = if style.strip_leading_hardline {
                strip_one_leading_hardline(first)
            } else {
                first
            };
            let tail = Doc::concat(vec![start, Doc::text(GAP), second, Doc::text(GAP), third]);
            Doc::concat(vec![
                Doc::text(rune),
                break_then_continuation(break_doc.clone(), tail, indent, style.nest_continuation),
            ])
        }
        RuneBreak::AfterFirst => {
            let continuation = if style.strip_leading_hardline {
                strip_one_leading_hardline(second)
            } else {
                second
            };
            let tail = Doc::concat(vec![continuation, Doc::text("  "), third]);
            Doc::concat(vec![
                Doc::text(rune),
                Doc::text(GAP),
                first,
                break_then_continuation(break_doc, tail, indent, style.nest_continuation),
            ])
        }
        RuneBreak::AfterSecond => {
            let continuation = if style.strip_leading_hardline {
                strip_one_leading_hardline(third)
            } else {
                third
            };
            Doc::concat(vec![
                Doc::text(rune),
                Doc::text(GAP),
                first,
                Doc::text(GAP),
                second,
                break_then_continuation(break_doc, continuation, indent, style.nest_continuation),
            ])
        }
    };

    let body = match style.line_mode {
        RuneLineMode::FitOrBreak => Doc::group(Doc::flat_alt(broken, flat)),
        RuneLineMode::AlwaysBreak => broken,
    };

    with_prefix(style.prefix, body)
}

/// Format a 4-child rune using a configurable line-breaking style.
pub fn rune4(
    rune: &str,
    first: Doc,
    second: Doc,
    third: Doc,
    fourth: Doc,
    indent: i32,
    style: RuneStyle,
) -> Doc {
    let flat = Doc::concat(vec![
        Doc::text(rune),
        Doc::text(GAP),
        first.clone(),
        Doc::text(GAP),
        second.clone(),
        Doc::text(GAP),
        third.clone(),
        Doc::text(GAP),
        fourth.clone(),
    ]);

    let break_doc = match style.line_mode {
        RuneLineMode::FitOrBreak => Doc::line(),
        RuneLineMode::AlwaysBreak => Doc::hardline(),
    };

    let broken = match style.break_mode {
        RuneBreak::BeforeFirst => {
            let start = if style.strip_leading_hardline {
                strip_one_leading_hardline(first)
            } else {
                first
            };
            let tail = Doc::concat(vec![
                start,
                Doc::text(GAP),
                second,
                Doc::text(GAP),
                third,
                Doc::text(GAP),
                fourth,
            ]);
            Doc::concat(vec![
                Doc::text(rune),
                break_then_continuation(break_doc.clone(), tail, indent, style.nest_continuation),
            ])
        }
        RuneBreak::AfterFirst => {
            let continuation = if style.strip_leading_hardline {
                strip_one_leading_hardline(second)
            } else {
                second
            };
            let tail = Doc::concat(vec![
                continuation,
                Doc::text(GAP),
                third,
                Doc::text(GAP),
                fourth,
            ]);
            Doc::concat(vec![
                Doc::text(rune),
                Doc::text(GAP),
                first,
                break_then_continuation(break_doc, tail, indent, style.nest_continuation),
            ])
        }
        RuneBreak::AfterSecond => {
            let continuation = if style.strip_leading_hardline {
                strip_one_leading_hardline(third)
            } else {
                third
            };
            let tail = Doc::concat(vec![continuation, Doc::text(GAP), fourth]);
            Doc::concat(vec![
                Doc::text(rune),
                Doc::text(GAP),
                first,
                Doc::text(GAP),
                second,
                break_then_continuation(break_doc, tail, indent, style.nest_continuation),
            ])
        }
    };

    let body = match style.line_mode {
        RuneLineMode::FitOrBreak => Doc::group(Doc::flat_alt(broken, flat)),
        RuneLineMode::AlwaysBreak => broken,
    };

    with_prefix(style.prefix, body)
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
