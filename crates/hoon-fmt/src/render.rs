//! Renderer that converts Doc to String with line fitting.
//!
//! Uses Wadler's algorithm to decide when to break lines based on available width.

use crate::config::FormatterConfig;
use crate::doc::Doc;
use std::rc::Rc;

/// Rendering mode - whether we're trying to fit on one line or not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Normal mode - break at Line and Gap as needed.
    Break,
    /// Flat mode - try to fit everything on one line.
    Flat,
}

/// A command in the rendering stack.
#[derive(Debug, Clone)]
struct Cmd {
    indent: i32,
    mode: Mode,
    doc: Rc<Doc>,
}

impl Cmd {
    fn new(indent: i32, mode: Mode, doc: &Doc) -> Self {
        Self {
            indent,
            mode,
            doc: Rc::new(doc.clone()),
        }
    }
}

/// Renderer that produces formatted output.
pub struct Renderer {
    config: FormatterConfig,
}

impl Renderer {
    /// Create a new renderer with the given configuration.
    pub fn new(config: FormatterConfig) -> Self {
        Self { config }
    }

    /// Render a document to a string.
    pub fn render(&self, doc: &Doc) -> String {
        let mut output = String::new();
        let mut col = 0i32;
        let mut stack: Vec<Cmd> = vec![Cmd::new(0, Mode::Break, doc)];

        while let Some(cmd) = stack.pop() {
            match cmd.doc.as_ref() {
                Doc::Nil => {}

                Doc::Text(s) => {
                    output.push_str(s);
                    col += s.len() as i32;
                }

                Doc::Line => {
                    if cmd.mode == Mode::Flat {
                        output.push(' ');
                        col += 1;
                    } else {
                        output.push('\n');
                        let indent = cmd.indent.max(0) as usize;
                        output.push_str(&" ".repeat(indent));
                        col = cmd.indent;
                    }
                }

                Doc::SoftLine => {
                    if cmd.mode == Mode::Flat {
                        // Nothing in flat mode
                    } else {
                        output.push('\n');
                        let indent = cmd.indent.max(0) as usize;
                        output.push_str(&" ".repeat(indent));
                        col = cmd.indent;
                    }
                }

                Doc::Gap => {
                    // Break to new line if we're past max_width, otherwise 2-space gap
                    if col >= self.config.max_width as i32 {
                        output.push('\n');
                        let indent = cmd.indent.max(0) as usize;
                        output.push_str(&" ".repeat(indent));
                        col = cmd.indent;
                    } else {
                        output.push_str("  ");
                        col += 2;
                    }
                }

                Doc::HardLine => {
                    output.push('\n');
                    let indent = cmd.indent.max(0) as usize;
                    output.push_str(&" ".repeat(indent));
                    col = cmd.indent;
                }

                Doc::Concat(docs) => {
                    // Push in reverse order so we process left to right
                    for d in docs.iter().rev() {
                        stack.push(Cmd::new(cmd.indent, cmd.mode, d));
                    }
                }

                Doc::Nest(i, inner) => {
                    stack.push(Cmd::new(cmd.indent + i, cmd.mode, inner.as_ref()));
                }

                Doc::Group(inner) => {
                    if cmd.mode == Mode::Flat {
                        stack.push(Cmd::new(cmd.indent, Mode::Flat, inner.as_ref()));
                    } else {
                        // Try to fit the group on one line
                        if self.fits(
                            self.config.max_width as i32 - col,
                            &[(cmd.indent, Mode::Flat, inner.as_ref())],
                        ) {
                            stack.push(Cmd::new(cmd.indent, Mode::Flat, inner.as_ref()));
                        } else {
                            stack.push(Cmd::new(cmd.indent, Mode::Break, inner.as_ref()));
                        }
                    }
                }

                Doc::FlatAlt(normal, flat) => {
                    if cmd.mode == Mode::Flat {
                        stack.push(Cmd::new(cmd.indent, cmd.mode, flat.as_ref()));
                    } else {
                        stack.push(Cmd::new(cmd.indent, cmd.mode, normal.as_ref()));
                    }
                }

                Doc::Backstep(dedent, inner) => {
                    stack.push(Cmd::new(cmd.indent + dedent, cmd.mode, inner.as_ref()));
                }

                Doc::Align(inner) => {
                    // Align subsequent lines with current column
                    stack.push(Cmd::new(col, cmd.mode, inner.as_ref()));
                }
            }
        }

        output
    }

    /// Check if a sequence of commands fits within the given width.
    fn fits(&self, mut width: i32, cmds: &[(i32, Mode, &Doc)]) -> bool {
        let mut stack: Vec<(i32, Mode, &Doc)> = cmds.iter().rev().cloned().collect();

        while width >= 0 {
            let Some((indent, mode, doc)) = stack.pop() else {
                return true;
            };

            match doc {
                Doc::Nil => {}

                Doc::Text(s) => {
                    width -= s.len() as i32;
                }

                Doc::Line => {
                    if mode == Mode::Flat {
                        width -= 1;
                    } else {
                        // Line break - fits
                        return true;
                    }
                }

                Doc::SoftLine => {
                    if mode != Mode::Flat {
                        return true;
                    }
                }

                Doc::Gap => {
                    if mode == Mode::Flat {
                        width -= 2;
                    } else {
                        return true;
                    }
                }

                Doc::HardLine => {
                    // Hard line always breaks
                    return true;
                }

                Doc::Concat(docs) => {
                    for d in docs.iter().rev() {
                        stack.push((indent, mode, d));
                    }
                }

                Doc::Nest(i, inner) => {
                    stack.push((indent + i, mode, inner.as_ref()));
                }

                Doc::Group(inner) => {
                    // In fits check, always try flat mode for groups
                    stack.push((indent, Mode::Flat, inner.as_ref()));
                }

                Doc::FlatAlt(_normal, flat) => {
                    // In fits check, use flat variant
                    stack.push((indent, mode, flat.as_ref()));
                }

                Doc::Backstep(dedent, inner) => {
                    stack.push((indent + dedent, mode, inner.as_ref()));
                }

                Doc::Align(inner) => {
                    // For fits check, indent doesn't matter much
                    stack.push((indent, mode, inner.as_ref()));
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_text() {
        let config = FormatterConfig::default();
        let renderer = Renderer::new(config);
        let doc = Doc::text("hello");
        assert_eq!(renderer.render(&doc), "hello");
    }

    #[test]
    fn test_concat() {
        let config = FormatterConfig::default();
        let renderer = Renderer::new(config);
        let doc = Doc::concat(vec![Doc::text("hello"), Doc::text(" "), Doc::text("world")]);
        assert_eq!(renderer.render(&doc), "hello world");
    }

    #[test]
    fn test_line_break() {
        let config = FormatterConfig::default();
        let renderer = Renderer::new(config);
        let doc = Doc::concat(vec![Doc::text("hello"), Doc::line(), Doc::text("world")]);
        assert_eq!(renderer.render(&doc), "hello\nworld");
    }

    #[test]
    fn test_nested_indent() {
        let config = FormatterConfig::default();
        let renderer = Renderer::new(config);
        let doc = Doc::concat(vec![
            Doc::text("if"),
            Doc::nest(2, Doc::concat(vec![Doc::line(), Doc::text("then")])),
        ]);
        assert_eq!(renderer.render(&doc), "if\n  then");
    }

    #[test]
    fn test_group_fits() {
        let config = FormatterConfig::default();
        let renderer = Renderer::new(config);
        let doc = Doc::group(Doc::concat(vec![
            Doc::text("a"),
            Doc::line(),
            Doc::text("b"),
        ]));
        // Should fit on one line as "a b"
        assert_eq!(renderer.render(&doc), "a b");
    }

    #[test]
    fn test_group_breaks() {
        let config = FormatterConfig::default().with_max_width(5);
        let renderer = Renderer::new(config);
        let doc = Doc::group(Doc::concat(vec![
            Doc::text("hello"),
            Doc::line(),
            Doc::text("world"),
        ]));
        // "hello world" is 11 chars, won't fit in 5
        assert_eq!(renderer.render(&doc), "hello\nworld");
    }
}
