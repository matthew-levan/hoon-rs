pub mod ast;
pub mod runes;
pub mod utils;

extern crate self as parser;

#[path = "parser_core.rs"]
mod parser_core;

use std::sync::Arc;

use chumsky::Parser;
use parser::ast::hoon::{Hoon, Path};
use parser::utils::LineMap;
pub use parser_core::parser as native_parser;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ImportKind {
    Plus,
    Minus,
    Equals,
    Star,
    Hash,
    Question,
    Percent,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImportDecl {
    pub kind: ImportKind,
    pub raw: String,
    pub target: Option<String>,
    pub bindings: Vec<ImportBinding>,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: u64,
    pub start_col: u64,
    pub end_line: u64,
    pub end_col: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImportBinding {
    pub visible_symbol: Option<String>,
    pub target: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParserError {
    pub message: String,
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DefinitionKind {
    Arm,
    Type,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DefinitionDecl {
    pub kind: DefinitionKind,
    pub name: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: u64,
    pub start_col: u64,
    pub end_line: u64,
    pub end_col: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LocalBindingKind {
    TisFas,
    TisGar,
    BarTis,
    BarCab,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LocalBindingDecl {
    pub kind: LocalBindingKind,
    pub name: String,
    pub declared_type: Option<String>,
    pub value: Option<String>,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: u64,
    pub start_col: u64,
    pub end_line: u64,
    pub end_col: u64,
    pub type_start_byte: Option<usize>,
    pub type_end_byte: Option<usize>,
    pub value_start_byte: Option<usize>,
    pub value_end_byte: Option<usize>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ParseWithMetadataResult {
    pub ast: Option<Hoon>,
    pub errors: Vec<ParserError>,
    pub imports: Vec<ImportDecl>,
    pub definitions: Vec<DefinitionDecl>,
}

pub fn parse_with_metadata(source: &str, wer: Path, bug: bool) -> ParseWithMetadataResult {
    let linemap = Arc::new(LineMap::new(source));
    let parse_result = native_parser(wer, bug, linemap, false)
        .parse(source)
        .into_result();
    let imports = extract_imports(source);
    let definitions = extract_definitions(source);

    match parse_result {
        Ok(ast) => ParseWithMetadataResult {
            ast: Some(ast),
            errors: Vec::new(),
            imports,
            definitions,
        },
        Err(errors) => ParseWithMetadataResult {
            ast: None,
            errors: errors
                .into_iter()
                .map(|err| {
                    let span = err.span().into_range();
                    ParserError {
                        message: err.reason().to_string(),
                        start_byte: span.start,
                        end_byte: span.end,
                    }
                })
                .collect(),
            imports,
            definitions,
        },
    }
}

pub fn definitions_with_metadata(source: &str) -> Vec<DefinitionDecl> {
    extract_definitions(source)
}

pub fn local_bindings_with_metadata(source: &str) -> Vec<LocalBindingDecl> {
    extract_local_bindings(source)
}

fn extract_imports(source: &str) -> Vec<ImportDecl> {
    #[derive(Clone)]
    struct LineInfo {
        line_number: usize,
        start_byte: usize,
        end_byte: usize,
        raw: String,
        trimmed: String,
        leading_ws: usize,
    }

    let mut lines = Vec::new();
    let mut byte_cursor = 0usize;
    for (line_idx, line) in source.split_inclusive('\n').enumerate() {
        let line_start = byte_cursor;
        let line_end = line_start + line.len();
        byte_cursor = line_end;
        let trimmed = line.trim_start().to_string();
        let leading_ws = line.len().saturating_sub(trimmed.len());
        lines.push(LineInfo {
            line_number: line_idx + 1,
            start_byte: line_start,
            end_byte: line_end,
            raw: line.to_string(),
            trimmed,
            leading_ws,
        });
    }

    let mut out = Vec::new();
    let mut seen_import = false;
    let mut idx = 0usize;

    while idx < lines.len() {
        let line = &lines[idx];
        let trimmed = line.trimmed.trim_end();

        if trimmed.is_empty() || trimmed.starts_with("::") {
            if !seen_import {
                idx += 1;
                continue;
            }
            break;
        }

        let Some((mut kind, mut target, mut bindings)) = parse_import_line(trimmed) else {
            if seen_import {
                break;
            }
            idx += 1;
            continue;
        };

        let mut raw = trimmed.to_string();
        let mut end_byte = line.end_byte;
        let mut end_line = line.line_number;
        let mut end_col = (line.raw.len() + 1) as u64;
        let start_line = line.line_number;
        let start_col = (line.leading_ws + 1) as u64;
        let start_byte = line.start_byte + line.leading_ws;

        // Continue parsing indented import list lines, e.g.:
        // /+  dbug,
        //     sr=sortug,
        //     lib=forum
        let mut lookahead = idx + 1;
        while lookahead < lines.len() {
            let next = &lines[lookahead];
            let next_trimmed = next.trimmed.trim_end();
            if next_trimmed.is_empty() || next_trimmed.starts_with("::") {
                break;
            }
            if parse_import_line(next_trimmed).is_some() {
                break;
            }
            if next.leading_ws == 0 {
                break;
            }

            let continuation = next_trimmed.trim_end_matches(',');
            if !continuation.is_empty() {
                if !raw.trim_end().ends_with(',') {
                    raw.push(',');
                }
                raw.push(' ');
                raw.push_str(continuation.trim_start_matches(',').trim());
            }
            end_byte = next.end_byte;
            end_line = next.line_number;
            end_col = (next.raw.len() + 1) as u64;
            lookahead += 1;
        }

        if lookahead > idx + 1 {
            if let Some((k2, t2, b2)) = parse_import_line(&raw) {
                kind = k2;
                target = t2;
                bindings = b2;
            }
            idx = lookahead - 1;
        }

        seen_import = true;

        out.push(ImportDecl {
            kind,
            raw,
            target,
            bindings,
            start_byte,
            end_byte,
            start_line: start_line as u64,
            start_col,
            end_line: end_line as u64,
            end_col,
        });
        idx += 1;
    }

    out
}

fn parse_import_line(trimmed: &str) -> Option<(ImportKind, Option<String>, Vec<ImportBinding>)> {
    let mut chars = trimmed.chars();
    if chars.next()? != '/' {
        return None;
    }

    let kind = match chars.next()? {
        '+' => ImportKind::Plus,
        '-' => ImportKind::Minus,
        '=' => ImportKind::Equals,
        '*' => ImportKind::Star,
        '#' => ImportKind::Hash,
        '?' => ImportKind::Question,
        '%' => ImportKind::Percent,
        _ => return None,
    };

    let rest = chars.as_str().trim();
    let bindings = import_bindings_for_line(kind.clone(), rest);
    let target = bindings.first().map(|b| b.target.clone());
    Some((kind, target, bindings))
}

fn import_bindings_for_line(kind: ImportKind, body: &str) -> Vec<ImportBinding> {
    match kind {
        ImportKind::Equals => parse_equals_bindings(body),
        ImportKind::Plus | ImportKind::Minus => parse_list_bindings(body),
        _ => Vec::new(),
    }
}

fn parse_equals_bindings(body: &str) -> Vec<ImportBinding> {
    let parts = body.split_whitespace().collect::<Vec<_>>();
    if parts.is_empty() {
        return Vec::new();
    }
    if parts.len() == 1 {
        let target = clean_import_atom(parts[0]);
        if target.is_empty() {
            return Vec::new();
        }
        let visible_symbol = if target.starts_with('*') {
            None
        } else {
            Some(import_name_from_target(&target).to_string())
        };
        return vec![ImportBinding {
            visible_symbol,
            target,
        }];
    }

    let lhs = clean_import_atom(parts[0]);
    let rhs = clean_import_atom(parts[1]);
    if rhs.is_empty() {
        return Vec::new();
    }
    let visible_symbol = if lhs == "*" || lhs.starts_with('*') || lhs.is_empty() {
        None
    } else {
        Some(lhs)
    };

    vec![ImportBinding {
        visible_symbol,
        target: rhs,
    }]
}

fn parse_list_bindings(body: &str) -> Vec<ImportBinding> {
    split_list_atoms(body)
        .into_iter()
        .filter_map(|token| {
            let (visible_symbol, target) = if let Some((lhs, rhs)) = token.split_once('=') {
                let lhs = clean_import_atom(lhs);
                let rhs = clean_import_atom(rhs);
                if rhs.is_empty() {
                    return None;
                }
                let visible_symbol = if lhs == "*" || lhs.starts_with('*') || lhs.is_empty() {
                    None
                } else {
                    Some(lhs)
                };
                (visible_symbol, rhs)
            } else {
                let visible_symbol = if token.starts_with('*') {
                    None
                } else {
                    Some(import_name_from_target(&token).to_string())
                };
                let target = if token.starts_with('*') {
                    token.trim_start_matches('*').to_string()
                } else {
                    token
                };
                (visible_symbol, target)
            };

            Some(ImportBinding {
                visible_symbol,
                target,
            })
        })
        .collect()
}

fn clean_import_atom(s: &str) -> String {
    s.trim()
        .trim_matches(|c: char| {
            c == ',' || c == ';' || c == ')' || c == '(' || c == '"' || c == '\''
        })
        .to_string()
}

fn split_list_atoms(body: &str) -> Vec<String> {
    let body = body.split("::").next().unwrap_or(body);
    let mut out = Vec::new();
    for part in body.split(',') {
        let cleaned_part = clean_import_atom(part);
        if cleaned_part.is_empty() {
            continue;
        }
        if cleaned_part.contains('=') {
            out.push(cleaned_part);
            continue;
        }
        for atom in cleaned_part.split_whitespace() {
            let atom = clean_import_atom(atom);
            if !atom.is_empty() {
                out.push(atom);
            }
        }
    }
    out
}

fn import_name_from_target(target: &str) -> &str {
    let without_ext = target.strip_suffix(".hoon").unwrap_or(target);
    without_ext.rsplit('/').next().unwrap_or(without_ext)
}

fn extract_definitions(source: &str) -> Vec<DefinitionDecl> {
    let mut out = Vec::new();
    let mut byte_cursor = 0usize;

    for (line_idx, line) in source.split_inclusive('\n').enumerate() {
        let line_start = byte_cursor;
        let line_end = line_start + line.len();
        byte_cursor = line_end;

        let trimmed = line.trim_start();
        let (kind, after_sig) = if let Some(rest) = trimmed.strip_prefix("++") {
            (DefinitionKind::Arm, rest)
        } else if let Some(rest) = trimmed.strip_prefix("+$") {
            (DefinitionKind::Type, rest)
        } else {
            continue;
        };

        let leading_ws = line.len().saturating_sub(trimmed.len());
        let inner_ws = after_sig.len().saturating_sub(after_sig.trim_start().len());
        let rest = after_sig.trim_start();
        if rest.is_empty() {
            continue;
        }

        let name_len = rest
            .bytes()
            .take_while(|b| matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_'))
            .count();
        if name_len == 0 {
            continue;
        }

        let name = rest[..name_len].to_string();
        let start_byte = line_start + leading_ws + 2 + inner_ws;
        let end_byte = start_byte + name_len;
        let start_col = (leading_ws + 2 + inner_ws + 1) as u64;
        let end_col = (leading_ws + 2 + inner_ws + name_len + 1) as u64;
        let line_no = (line_idx + 1) as u64;

        out.push(DefinitionDecl {
            kind,
            name,
            start_byte,
            end_byte,
            start_line: line_no,
            start_col,
            end_line: line_no,
            end_col,
        });
    }

    out
}

fn extract_local_bindings(source: &str) -> Vec<LocalBindingDecl> {
    let mut out = Vec::new();
    let mut byte_cursor = 0usize;

    for (line_idx, line) in source.split_inclusive('\n').enumerate() {
        let line_start = byte_cursor;
        let line_end = line_start + line.len();
        byte_cursor = line_end;

        let trimmed = line.trim_start();
        if trimmed.starts_with("::") {
            continue;
        }

        let (kind, sig, after_sig) = if let Some(rest) = trimmed.strip_prefix("=/") {
            (LocalBindingKind::TisFas, "=/", rest)
        } else if let Some(rest) = trimmed.strip_prefix("=*") {
            (LocalBindingKind::TisGar, "=*", rest)
        } else if let Some(rest) = trimmed.strip_prefix("|=") {
            (LocalBindingKind::BarTis, "|=", rest)
        } else if let Some(rest) = trimmed.strip_prefix("|_") {
            (LocalBindingKind::BarCab, "|_", rest)
        } else {
            continue;
        };

        let leading_ws = line.len().saturating_sub(trimmed.len());
        let inner_ws = after_sig.len().saturating_sub(after_sig.trim_start().len());
        let rest = after_sig.trim_start();
        if rest.is_empty() {
            continue;
        }

        if matches!(kind, LocalBindingKind::BarTis | LocalBindingKind::BarCab) {
            if let Some((sample_part, sample_offset)) = leading_sample(rest) {
                let sample_abs_start =
                    line_start + leading_ws + sig.len() + inner_ws + sample_offset;
                for parsed in parse_sample_bindings(sample_part, sample_abs_start) {
                    out.push(LocalBindingDecl {
                        kind,
                        name: parsed.name,
                        declared_type: parsed.declared_type,
                        value: None,
                        start_byte: parsed.start_byte,
                        end_byte: parsed.end_byte,
                        start_line: (line_idx + 1) as u64,
                        start_col: (parsed.start_byte.saturating_sub(line_start) + 1) as u64,
                        end_line: (line_idx + 1) as u64,
                        end_col: (parsed.end_byte.saturating_sub(line_start) + 1) as u64,
                        type_start_byte: parsed.type_start_byte,
                        type_end_byte: parsed.type_end_byte,
                        value_start_byte: None,
                        value_end_byte: None,
                    });
                }
            }
            continue;
        }

        let mut parts = rest.splitn(2, char::is_whitespace);
        let name_part = parts.next().unwrap_or_default().trim();
        let value = parts
            .next()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(ToOwned::to_owned);

        let (name, declared_type, type_span_in_name) =
            if let Some((n, ty)) = name_part.split_once('=') {
                let clean_name = n.trim();
                let clean_ty = ty.trim();
                let ty_start = name_part.find('=').map(|eq| eq + 1).unwrap_or(0);
                (
                    clean_name.to_string(),
                    (!clean_ty.is_empty()).then_some(clean_ty.to_string()),
                    if clean_ty.is_empty() {
                        None
                    } else {
                        Some((ty_start, ty_start + clean_ty.len()))
                    },
                )
            } else {
                (name_part.to_string(), None, None)
            };

        if name.is_empty() {
            continue;
        }

        let sig_len = sig.len();
        let name_start = line_start + leading_ws + sig_len + inner_ws;
        let name_end = name_start + name.len();

        let (type_start_byte, type_end_byte) = if let Some((s, e)) = type_span_in_name {
            let start = name_start + s;
            let end = name_start + e;
            (Some(start), Some(end))
        } else {
            (None, None)
        };

        let value_start_byte = value.as_ref().map(|_v| {
            let after_name = &line[name_end.saturating_sub(line_start)..];
            let ws = after_name
                .len()
                .saturating_sub(after_name.trim_start().len());
            name_end + ws
        });
        let value_end_byte = value_start_byte
            .zip(value.as_ref())
            .map(|(s, v)| s + v.len());

        let line_no = (line_idx + 1) as u64;
        let start_col = (leading_ws + sig_len + inner_ws + 1) as u64;
        let end_col = (start_col as usize + name.len()) as u64;

        out.push(LocalBindingDecl {
            kind,
            name,
            declared_type,
            value,
            start_byte: name_start,
            end_byte: name_end,
            start_line: line_no,
            start_col,
            end_line: line_no,
            end_col,
            type_start_byte,
            type_end_byte,
            value_start_byte,
            value_end_byte,
        });
    }

    out
}

#[derive(Debug)]
struct ParsedSampleBinding {
    name: String,
    declared_type: Option<String>,
    start_byte: usize,
    end_byte: usize,
    type_start_byte: Option<usize>,
    type_end_byte: Option<usize>,
}

fn leading_sample(rest: &str) -> Option<(&str, usize)> {
    let trimmed = rest.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    let start_offset = rest.len().saturating_sub(trimmed.len());
    if let Some(after_eq) = trimmed.strip_prefix("=(") {
        let mut depth = 1usize;
        for (idx, ch) in after_eq.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        let end = idx + 3;
                        return Some((&trimmed[..end], start_offset));
                    }
                }
                _ => {}
            }
        }
        return Some((trimmed, start_offset));
    }
    if let Some(after) = trimmed.strip_prefix('[') {
        let mut depth = 1usize;
        for (idx, ch) in after.char_indices() {
            match ch {
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    if depth == 0 {
                        let end = idx + 2;
                        return Some((&trimmed[..end], start_offset));
                    }
                }
                _ => {}
            }
        }
        return Some((trimmed, start_offset));
    }
    if let Some(after) = trimmed.strip_prefix('(') {
        let mut depth = 1usize;
        for (idx, ch) in after.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        let end = idx + 2;
                        return Some((&trimmed[..end], start_offset));
                    }
                }
                _ => {}
            }
        }
        return Some((trimmed, start_offset));
    }

    let end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    Some((&trimmed[..end], start_offset))
}

fn parse_sample_bindings(sample: &str, sample_abs_start: usize) -> Vec<ParsedSampleBinding> {
    let mut out = Vec::new();
    let mut cursor = 0usize;
    while cursor < sample.len() {
        let Some(start_rel) = sample[cursor..]
            .char_indices()
            .find_map(|(i, ch)| is_ident_char(ch).then_some(cursor + i))
        else {
            break;
        };

        let mut end_rel = start_rel;
        while let Some(ch) = sample[end_rel..].chars().next() {
            if !is_ident_char(ch) {
                break;
            }
            end_rel += ch.len_utf8();
        }

        let name = sample[start_rel..end_rel].to_string();
        let mut ty = None;
        let mut ty_rel_start = None;
        let mut ty_rel_end = None;
        let mut check = end_rel;
        while let Some(ch) = sample[check..].chars().next() {
            if ch.is_whitespace() {
                check += ch.len_utf8();
                continue;
            }
            if ch == '=' || ch == ':' {
                let sep_len = ch.len_utf8();
                let ty_start = check + sep_len;
                let mut ty_end = ty_start;
                while let Some(tch) = sample[ty_end..].chars().next() {
                    if tch.is_whitespace() || matches!(tch, ',' | ']' | ')' | '[' | '(') {
                        break;
                    }
                    ty_end += tch.len_utf8();
                }
                if ty_end > ty_start {
                    let rhs = sample[ty_start..ty_end].to_string();
                    ty = Some(if ch == ':' {
                        format!("{name}:{rhs}")
                    } else {
                        rhs
                    });
                    ty_rel_start = Some(ty_start);
                    ty_rel_end = Some(ty_end);
                }
            }
            break;
        }

        out.push(ParsedSampleBinding {
            name,
            declared_type: ty,
            start_byte: sample_abs_start + start_rel,
            end_byte: sample_abs_start + end_rel,
            type_start_byte: ty_rel_start.map(|s| sample_abs_start + s),
            type_end_byte: ty_rel_end.map(|e| sample_abs_start + e),
        });
        cursor = ty_rel_end.unwrap_or(end_rel);
    }
    out
}

fn is_ident_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'
}

#[cfg(test)]
mod tests {
    use super::{local_bindings_with_metadata, LocalBindingKind};

    #[test]
    fn extracts_bar_samples_as_local_bindings() {
        let src = "|_  =bowl:gall\n|=  [is-ted=? uidt=@t]\n";
        let bindings = local_bindings_with_metadata(src);

        assert!(bindings.iter().any(|b| {
            b.kind == LocalBindingKind::BarCab
                && b.name == "bowl"
                && b.declared_type.as_deref() == Some("bowl:gall")
        }));
        assert!(bindings.iter().any(|b| {
            b.kind == LocalBindingKind::BarTis
                && b.name == "uidt"
                && b.declared_type.as_deref() == Some("@t")
        }));
    }
}
