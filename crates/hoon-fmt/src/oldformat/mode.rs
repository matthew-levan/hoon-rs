//! Shared formatting mode.

/// Format mode: Tall (default, top-level) or Wide (inside brackets/parens).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FormatMode {
    Tall,
    Wide,
}
