//! Document intermediate representation for pretty printing.
//!
//! This is a Wadler-Lindig style document algebra that supports:
//! - Automatic line breaking based on available width
//! - Nested indentation
//! - Group flattening (tall vs wide forms)
//! - Backstep indentation (Hoon's signature style)

use std::rc::Rc;

/// A document is an intermediate representation between the AST and the final string output.
/// Documents describe *what* to print while deferring *how* to break lines until rendering.
#[derive(Debug, Clone)]
pub enum Doc {
    /// Empty document.
    Nil,
    /// A literal string (should not contain newlines).
    Text(String),
    /// A line break. In flat mode, renders as a single space.
    Line,
    /// A line break that renders as nothing in flat mode (for separating elements).
    SoftLine,
    /// A gap - at least 2 spaces in tall mode, single space in wide mode.
    Gap,
    /// A hard line break that always renders as newline (never flattened).
    HardLine,
    /// Concatenation of multiple documents.
    Concat(Vec<Doc>),
    /// Increase indentation for the nested document.
    Nest(i32, Rc<Doc>),
    /// A group that may be flattened to fit on one line.
    Group(Rc<Doc>),
    /// Choose between two documents based on whether we're in flat mode.
    /// First is the "normal" (broken) form, second is the "flat" form.
    FlatAlt(Rc<Doc>, Rc<Doc>),
    /// Backstep: dedent the last child to align with the rune.
    /// The i32 is the amount to dedent (typically negative).
    Backstep(i32, Rc<Doc>),
    /// Align subsequent lines with the current column.
    Align(Rc<Doc>),
}

impl Doc {
    /// Create an empty document.
    pub fn nil() -> Doc {
        Doc::Nil
    }

    /// Create a text document.
    pub fn text<S: Into<String>>(s: S) -> Doc {
        Doc::Text(s.into())
    }

    /// Create a line break (space when flattened).
    pub fn line() -> Doc {
        Doc::Line
    }

    /// Create a soft line break (nothing when flattened).
    pub fn softline() -> Doc {
        Doc::SoftLine
    }

    /// Create a gap (2+ spaces in tall, 1 space in wide).
    pub fn gap() -> Doc {
        Doc::Gap
    }

    /// Create a hard line break that never flattens.
    pub fn hardline() -> Doc {
        Doc::HardLine
    }

    /// Concatenate multiple documents.
    pub fn concat(docs: Vec<Doc>) -> Doc {
        // Flatten nested concats and remove nils
        let mut result = Vec::new();
        for doc in docs {
            match doc {
                Doc::Nil => {}
                Doc::Concat(inner) => result.extend(inner),
                other => result.push(other),
            }
        }
        if result.is_empty() {
            Doc::Nil
        } else if result.len() == 1 {
            result.pop().unwrap()
        } else {
            Doc::Concat(result)
        }
    }

    /// Nest a document with additional indentation.
    pub fn nest(indent: i32, doc: Doc) -> Doc {
        Doc::Nest(indent, Rc::new(doc))
    }

    /// Group a document (may be flattened).
    pub fn group(doc: Doc) -> Doc {
        Doc::Group(Rc::new(doc))
    }

    /// Choose between normal and flat forms.
    pub fn flat_alt(normal: Doc, flat: Doc) -> Doc {
        Doc::FlatAlt(Rc::new(normal), Rc::new(flat))
    }

    /// Backstep the last element.
    pub fn backstep(dedent: i32, doc: Doc) -> Doc {
        Doc::Backstep(dedent, Rc::new(doc))
    }

    /// Align content with current column.
    pub fn align(doc: Doc) -> Doc {
        Doc::Align(Rc::new(doc))
    }

    /// Join documents with a separator.
    pub fn join(sep: Doc, docs: Vec<Doc>) -> Doc {
        let mut result = Vec::with_capacity(docs.len() * 2);
        let mut first = true;
        for doc in docs {
            if !first {
                result.push(sep.clone());
            }
            first = false;
            result.push(doc);
        }
        Doc::concat(result)
    }

    /// Join documents with a line separator.
    pub fn lines(docs: Vec<Doc>) -> Doc {
        Doc::join(Doc::line(), docs)
    }

    /// Join documents with a gap separator.
    pub fn gaps(docs: Vec<Doc>) -> Doc {
        Doc::join(Doc::gap(), docs)
    }

    /// Wrap with parentheses.
    pub fn parens(doc: Doc) -> Doc {
        Doc::concat(vec![Doc::text("("), doc, Doc::text(")")])
    }

    /// Wrap with brackets.
    pub fn brackets(doc: Doc) -> Doc {
        Doc::concat(vec![Doc::text("["), doc, Doc::text("]")])
    }
}

// Convenience operators for building documents
impl std::ops::Add for Doc {
    type Output = Doc;

    fn add(self, rhs: Doc) -> Doc {
        Doc::concat(vec![self, rhs])
    }
}

/// Helper macro for building documents
#[macro_export]
macro_rules! docs {
    () => { $crate::doc::Doc::nil() };
    ($single:expr) => { $single };
    ($first:expr, $($rest:expr),+ $(,)?) => {
        $crate::doc::Doc::concat(vec![$first, $($rest),+])
    };
}
