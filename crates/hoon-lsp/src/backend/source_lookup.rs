use super::*;

pub(super) fn definition_value_for_location(
    current_uri: &Url,
    current_text: &str,
    symbol: &str,
    location: &Location,
) -> Option<DefinitionValue> {
    let text = if &location.uri == current_uri {
        current_text.to_string()
    } else {
        let path = location.uri.to_file_path().ok()?;
        fs::read_to_string(path).ok()?
    };

    definition_for_symbol_at_line(
        &text,
        symbol,
        usize::try_from(location.range.start.line).ok()?,
    )
    .or_else(|| arm_value_for_symbol(&text, symbol).map(DefinitionValue::Expr))
    .or_else(|| type_definition_for_symbol(&text, symbol).map(DefinitionValue::TypeDef))
}

pub(super) enum DefinitionValue {
    Expr(String),
    TypeDef(String),
}

fn definition_for_symbol_at_line(text: &str, symbol: &str, line_index: usize) -> Option<DefinitionValue> {
    let line = text.lines().nth(line_index)?;
    let header = parse_definition_header(line)?;
    if header.name != symbol {
        return None;
    }

    match header.kind {
        DefinitionKind::Arm => arm_value_from_line(text, line_index).map(DefinitionValue::Expr),
        DefinitionKind::Type => header
            .inline_value
            .or_else(|| type_definition_from_line(text, line_index))
            .map(DefinitionValue::TypeDef),
    }
}

pub(crate) fn field_type_from_location(
    current_uri: &Url,
    current_text: &str,
    location: &Location,
    type_name: &str,
    field: &str,
) -> Option<String> {
    let text = if &location.uri == current_uri {
        current_text.to_string()
    } else {
        let path = location.uri.to_file_path().ok()?;
        fs::read_to_string(path).ok()?
    };
    let line_index = usize::try_from(location.range.start.line).ok()?;
    let line = text.lines().nth(line_index)?;
    let header = parse_definition_header(line)?;
    if header.kind != DefinitionKind::Type || header.name != type_name {
        return None;
    }

    let type_block = definition_for_symbol_at_line(&text, type_name, line_index)?;
    let DefinitionValue::TypeDef(def_text) = type_block else {
        return None;
    };
    extract_field_type_from_type_definition(&def_text, field)
}

pub(crate) fn field_type_from_type_in_text(
    text: &str,
    type_name: &str,
    module: &str,
    field: &str,
) -> Option<String> {
    let lines = text.lines().collect::<Vec<_>>();
    for (idx, line) in lines.iter().enumerate() {
        let Some(header) = parse_definition_header(line) else {
            continue;
        };
        if header.kind != DefinitionKind::Type || header.name != type_name {
            continue;
        }
        if !line_index_is_member_of_module(text, idx, module) {
            continue;
        }
        let block = definition_for_symbol_at_line(text, type_name, idx)?;
        let DefinitionValue::TypeDef(def_text) = block else {
            continue;
        };
        if let Some(ty) = extract_field_type_from_type_definition(&def_text, field) {
            return Some(ty);
        }
    }
    None
}

fn extract_field_type_from_type_definition(definition: &str, field: &str) -> Option<String> {
    for line in definition.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("::") {
            continue;
        }
        let needle = format!("{field}=");
        if let Some(pos) = trimmed.find(&needle) {
            let after = &trimmed[pos + needle.len()..];
            let ty = type_expr_prefix(after);
            if !ty.is_empty() {
                return Some(ty);
            }
        }
    }
    None
}

fn type_expr_prefix(input: &str) -> String {
    let s = input.trim_start();
    if s.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    let mut depth_paren = 0usize;
    let mut depth_bracket = 0usize;
    for ch in s.chars() {
        match ch {
            '(' => {
                depth_paren += 1;
                out.push(ch);
            }
            ')' => {
                if depth_paren == 0 {
                    break;
                }
                depth_paren -= 1;
                out.push(ch);
                if depth_paren == 0 && depth_bracket == 0 {
                    break;
                }
            }
            '[' => {
                depth_bracket += 1;
                out.push(ch);
            }
            ']' => {
                if depth_bracket == 0 {
                    break;
                }
                depth_bracket -= 1;
                out.push(ch);
                if depth_paren == 0 && depth_bracket == 0 {
                    break;
                }
            }
            _ if ch.is_whitespace() && depth_paren == 0 && depth_bracket == 0 => break,
            _ if matches!(ch, ',' | ';') && depth_paren == 0 && depth_bracket == 0 => break,
            _ => out.push(ch),
        }
    }
    out.trim().to_string()
}

pub(crate) fn location_is_member_of_module(
    current_uri: &Url,
    current_text: &str,
    location: &Location,
    module: &str,
) -> bool {
    let text = if &location.uri == current_uri {
        current_text.to_string()
    } else {
        let Ok(path) = location.uri.to_file_path() else {
            return false;
        };
        let Ok(text) = fs::read_to_string(path) else {
            return false;
        };
        text
    };

    let line_index = usize::try_from(location.range.start.line).ok();
    let Some(line_index) = line_index else {
        return false;
    };
    let lines = text.lines().collect::<Vec<_>>();
    if line_index >= lines.len() {
        return false;
    }

    let def_indent = leading_whitespace(lines[line_index]);
    for idx in (0..line_index).rev() {
        let line = lines[idx];
        let Some(header) = parse_definition_header(line) else {
            continue;
        };
        let indent = leading_whitespace(line);
        if indent < def_indent {
            return header.name == module;
        }
    }
    false
}

fn line_index_is_member_of_module(text: &str, line_index: usize, module: &str) -> bool {
    let lines = text.lines().collect::<Vec<_>>();
    if line_index >= lines.len() {
        return false;
    }

    let def_indent = leading_whitespace(lines[line_index]);
    for idx in (0..line_index).rev() {
        let line = lines[idx];
        let Some(header) = parse_definition_header(line) else {
            continue;
        };
        let indent = leading_whitespace(line);
        if indent < def_indent {
            return header.name == module;
        }
    }
    false
}

fn leading_whitespace(line: &str) -> usize {
    line.len().saturating_sub(line.trim_start().len())
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum DefinitionKind {
    Arm,
    Type,
}

struct DefinitionHeader {
    kind: DefinitionKind,
    name: String,
    inline_value: Option<String>,
}

fn parse_definition_header(line: &str) -> Option<DefinitionHeader> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("::") {
        return None;
    }
    let (kind, rest) = if let Some(rest) = trimmed.strip_prefix("++") {
        (DefinitionKind::Arm, rest)
    } else if let Some(rest) = trimmed.strip_prefix("+$") {
        (DefinitionKind::Type, rest)
    } else {
        return None;
    };

    let mut parts = rest.trim_start().splitn(2, char::is_whitespace);
    let name = parts.next()?.trim();
    if name.is_empty() {
        return None;
    }
    let inline_value = parts
        .next()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned);

    Some(DefinitionHeader {
        kind,
        name: name.to_string(),
        inline_value,
    })
}

fn arm_value_from_line(text: &str, line_index: usize) -> Option<String> {
    let mut lines = text.lines().skip(line_index);
    let line = lines.next()?;
    let trimmed = line.trim_start();
    if !trimmed.starts_with("++") {
        return None;
    }
    let rest = trimmed.trim_start_matches('+').trim_start_matches('+').trim_start();
    let mut parts = rest.splitn(2, char::is_whitespace);
    let _name = parts.next()?.trim();
    let inline = parts.next().unwrap_or("").trim();
    if !inline.is_empty() {
        return Some(inline.to_string());
    }

    for next in lines {
        let next_trim = next.trim();
        if next_trim.is_empty() || next_trim.starts_with("::") {
            continue;
        }
        if is_top_level_definition_line(next) || next_trim == "--" {
            break;
        }
        return Some(next_trim.to_string());
    }
    None
}

pub(crate) fn arm_value_for_symbol(text: &str, symbol: &str) -> Option<String> {
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("++") {
            continue;
        }

        let rest = trimmed.trim_start_matches('+').trim_start_matches('+').trim_start();
        let mut parts = rest.splitn(2, char::is_whitespace);
        let name = parts.next()?.trim();
        if name != symbol {
            continue;
        }

        let inline = parts.next().unwrap_or("").trim();
        if !inline.is_empty() {
            return Some(inline.to_string());
        }

        for next in lines.by_ref() {
            let next_trim = next.trim();
            if next_trim.is_empty() || next_trim.starts_with("::") {
                continue;
            }
            if next_trim.starts_with("++") || next_trim == "--" {
                break;
            }
            return Some(next_trim.to_string());
        }
        return None;
    }
    None
}

fn type_definition_for_symbol(text: &str, symbol: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let Some(header) = parse_definition_header(line) else {
            continue;
        };
        if header.kind == DefinitionKind::Type && header.name == symbol {
            return header
                .inline_value
                .or_else(|| type_definition_from_line(text, i));
        }
    }
    None
}

fn type_definition_from_line(text: &str, line_index: usize) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    if line_index >= lines.len() {
        return None;
    }

    let mut out = Vec::new();
    for line in lines.iter().skip(line_index + 1) {
        let trimmed = line.trim_start();
        if trimmed == "--" {
            break;
        }
        if is_top_level_definition_line(line) {
            break;
        }
        out.push((*line).to_string());
    }

    if out.is_empty() {
        None
    } else {
        Some(out.join("\n"))
    }
}

fn is_top_level_definition_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    line.len() == trimmed.len()
        && (trimmed.starts_with("++") || trimmed.starts_with("+$"))
}
