//! Hoon code formatter.
//!
//! This crate provides a code formatter for the Hoon programming language.
//! It parses Hoon source code, converts it to an intermediate document representation,
//! and renders it with proper indentation and line breaking.
//!
//! # Example
//!
//! ```ignore
//! use formatter::{format_source, FormatterConfig};
//!
//! let source = "|=  a=@  (add a 1)";
//! let config = FormatterConfig::default();
//! let formatted = format_source(source, &config)?;
//! println!("{}", formatted);
//! ```

pub mod config;
pub mod doc;
pub mod format;
pub mod render;

pub use config::FormatterConfig;
pub use doc::Doc;
pub use render::Renderer;

use chumsky::Parser;
use format::hoon::format_hoon;
use parser::native_parser;
use std::sync::Arc;

/// Error type for formatter operations.
#[derive(Debug, thiserror::Error)]
pub enum FormatterError {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Format error: {0}")]
    FormatError(String),
}

/// Format Hoon source code.
///
/// # Arguments
///
/// * `source` - The Hoon source code to format
/// * `config` - The formatter configuration
///
/// # Returns
///
/// The formatted source code, or an error if parsing fails.
pub fn format_source(source: &str, config: &FormatterConfig) -> Result<String, FormatterError> {
    // Create line map for parser
    let linemap = Arc::new(parser::utils::LineMap::new(source));

    // Create a dummy path
    let wer = vec!["formatter".to_string()];

    // Parse the source with preserve_syntax=true for formatting
    let parser = native_parser(wer, false, linemap, true);

    let result = parser.parse(source);

    match result.into_result() {
        Ok(hoon) => {
            // Format the AST
            let doc = format_hoon(&hoon, config);

            // Render the document
            let renderer = Renderer::new(config.clone());
            Ok(renderer.render(&doc))
        }
        Err(errs) => {
            let messages: Vec<String> = errs
                .into_iter()
                .map(|e: chumsky::error::Rich<'_, char>| format!("{:?}", e.reason()))
                .collect();
            Err(FormatterError::ParseError(messages.join("; ")))
        }
    }
}

/// Format a Hoon AST directly (for when you already have a parsed AST).
pub fn format_hoon_ast(hoon: &parser::ast::hoon::Hoon, config: &FormatterConfig) -> String {
    let doc = format_hoon(hoon, config);
    let renderer = Renderer::new(config.clone());
    renderer.render(&doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_simple() {
        let config = FormatterConfig::default();
        // Just test that parsing and formatting doesn't crash
        let source = "%trivial";
        let result = format_source(source, &config);
        assert!(result.is_ok());
    }
}
