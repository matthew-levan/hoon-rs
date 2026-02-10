use std::path::Path;
use std::sync::Arc;

use chumsky::Parser;
use parser::native_parser;
use parser::utils::LineMap;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Url};

use crate::position_mapper::range_from_byte_offsets;

pub fn parse_diagnostics(uri: &Url, text: &str) -> Vec<Diagnostic> {
    let wer = wer_from_uri(uri);
    let linemap = Arc::new(LineMap::new(text));

    match native_parser(wer, false, linemap, false)
        .parse(text)
        .into_result()
    {
        Ok(_) => Vec::new(),
        Err(errors) => errors
            .into_iter()
            .map(|err| {
                let span = err.span().into_range();
                let range = range_from_byte_offsets(text, span.start, span.end);
                Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::ERROR),
                    source: Some("hoon-lsp".to_string()),
                    message: err.reason().to_string(),
                    ..Diagnostic::default()
                }
            })
            .collect(),
    }
}

fn wer_from_uri(uri: &Url) -> Vec<String> {
    if uri.scheme() == "file" {
        if let Ok(path) = uri.to_file_path() {
            return path
                .iter()
                .map(|part| part.to_string_lossy().into_owned())
                .collect();
        }
    }

    let fallback = uri.path();
    Path::new(fallback)
        .iter()
        .map(|part| part.to_string_lossy().into_owned())
        .collect()
}
