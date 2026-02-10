use std::collections::HashMap;

use super::*;
use self::source_lookup::definition_value_for_location;
use self::value_render::ParserValueRender;

#[path = "value_render.rs"]
mod value_render;
#[path = "source_lookup.rs"]
mod source_lookup;

pub(super) use self::source_lookup::{
    field_type_from_location, field_type_from_type_in_text, location_is_member_of_module,
};
pub(crate) use self::value_render::{parser_render_value, parser_render_value_with_scope};

pub(super) fn is_tilde_literal_at(text: &str, byte_offset: usize) -> bool {
    if text.is_empty() {
        return false;
    }
    let mut idx = byte_offset.min(text.len());
    if idx == text.len() && idx > 0 {
        idx -= 1;
    }
    text.as_bytes().get(idx).copied() == Some(b'~')
}

pub(super) fn symbol_hover_markdown(current_uri: &Url, current_text: &str, symbol: &str, matches: &[Location]) -> String {
    let first = &matches[0];
    if let Some(definition) = definition_value_for_location(current_uri, current_text, symbol, first) {
        let mut md = format!("**`{symbol}`**\n");
        match definition {
            source_lookup::DefinitionValue::Expr(expr) => {
                let render = parser_render_value_with_scope(&expr, Some(current_text));
                if render.as_ref().and_then(|r| r.ty.as_deref()) == Some("reference") {
                    let unresolved = render
                        .as_ref()
                        .and_then(|r| r.value.clone())
                        .unwrap_or_else(|| expr.trim().to_string());
                    md.push_str(&format!(
                        "\n- unresolved: could not resolve reference `{unresolved}` in current scope"
                    ));
                    return md;
                }
                let value = render
                    .as_ref()
                    .and_then(|r| r.value.clone())
                    .unwrap_or_else(|| expr.clone());
                if let Some(ty) = render.and_then(|r| r.ty) {
                    md.push_str(&format!("\n- type: `{ty}`"));
                }
                md.push_str(&format!("\n- value: `{value}`"));
            }
            source_lookup::DefinitionValue::TypeDef(definition) => {
                md.push_str("\n- type: `type`");
                md.push_str("\n- value:");
                md.push_str("\n```hoon\n");
                md.push_str(definition.trim_end());
                md.push_str("\n```");
            }
        }
        return md;
    }

    format!(
        "**`{symbol}`**\n\nDefinition found at `{}`:{}",
        first.uri,
        first.range.start.line + 1
    )
}

#[derive(Clone)]
pub(super) struct LocalBinding {
    pub(super) declared_type: Option<String>,
    pub(super) value: String,
}

pub(super) fn local_binding_hover_markdown(text: &str, cursor_byte: usize, symbol: &str) -> Option<String> {
    let bindings = collect_local_bindings_until(text, cursor_byte);
    let binding = bindings.get(symbol)?;
    let (resolved_value, inferred_type) = resolve_binding_value_and_type(symbol, &bindings, 0);
    let value = resolved_value.unwrap_or_else(|| binding.value.clone());
    let parser_render = if value.trim().is_empty() {
        None
    } else {
        parser_render_value(&value)
    };
    if parser_render.as_ref().and_then(|r| r.ty.as_deref()) == Some("reference")
        && !value.trim().is_empty()
    {
        let reference = parser_render
            .as_ref()
            .and_then(|r| r.value.clone())
            .unwrap_or_else(|| value.trim().to_string());
        return Some(unresolved_hover_markdown(symbol, &reference));
    }
    let rendered_value = parser_render
        .as_ref()
        .and_then(|r| r.value.clone())
        .unwrap_or_else(|| value.clone());
    let ty = binding
        .declared_type
        .clone()
        .or_else(|| parser_render.as_ref().and_then(|r| r.ty.clone()))
        .or(inferred_type);

    let mut md = format!("**`{symbol}`**");
    if let Some(ty) = ty {
        md.push_str(&format!("\n\n- type: `{ty}`"));
    }
    if !rendered_value.trim().is_empty() {
        md.push_str(&format!("\n- value: `{rendered_value}`"));
    }
    Some(md)
}

pub(super) fn local_binding_definition_location(
    uri: &Url,
    text: &str,
    cursor_byte: usize,
    symbol: &str,
) -> Option<Location> {
    let mut last = None;
    for binding in local_bindings_with_metadata(text) {
        if binding.start_byte > cursor_byte {
            continue;
        }
        if binding.name == symbol {
            last = Some(Location {
                uri: uri.clone(),
                range: range_from_byte_offsets(text, binding.start_byte, binding.end_byte),
            });
        }
    }
    last
}

pub(super) fn local_binding_type_symbol_at(text: &str, cursor_byte: usize) -> Option<String> {
    for binding in local_bindings_with_metadata(text) {
        let (Some(start), Some(end)) = (binding.type_start_byte, binding.type_end_byte) else {
            continue;
        };
        if cursor_byte >= start && cursor_byte < end {
            if let Some(ty) = binding.declared_type {
                return Some(ty);
            }
        }
    }
    None
}

pub(super) fn literal_hover_markdown(token: &str, render: &ParserValueRender) -> String {
    let mut md = format!("**`{}`**", token.trim());
    if let Some(ty) = &render.ty {
        md.push_str(&format!("\n\n- type: `{ty}`"));
    }
    if let Some(value) = &render.value {
        md.push_str(&format!("\n- value: `{value}`"));
    }
    md
}

pub(super) fn unresolved_hover_markdown(symbol: &str, reference: &str) -> String {
    format!(
        "**`{}`**\n\n- unresolved: could not resolve `{}` in current scope",
        symbol.trim(),
        reference.trim()
    )
}

pub(super) fn collect_local_bindings_until(text: &str, cursor_byte: usize) -> HashMap<String, LocalBinding> {
    let mut out = HashMap::new();
    let parsed = local_bindings_with_metadata(text);
    for binding in parsed {
        if binding.start_byte > cursor_byte {
            break;
        }
        let value = binding.value.unwrap_or_else(|| "".to_string());
        out.insert(
            binding.name.clone(),
            LocalBinding {
                declared_type: binding.declared_type,
                value,
            },
        );
    }
    out
}

pub(super) fn resolve_binding_value_and_type(
    symbol: &str,
    bindings: &HashMap<String, LocalBinding>,
    depth: usize,
) -> (Option<String>, Option<String>) {
    if depth > 8 {
        return (None, None);
    }
    let Some(binding) = bindings.get(symbol) else {
        return (None, None);
    };

    let declared = binding.declared_type.clone();
    if let Some(alias) = alias_symbol(&binding.value) {
        if alias != symbol {
            let (v, ty) = resolve_binding_value_and_type(alias, bindings, depth + 1);
            if v.is_some() {
                return (v, declared.or(ty));
            }
        }
    }

    (Some(binding.value.clone()), declared)
}

pub(super) fn alias_symbol(value: &str) -> Option<&str> {
    let v = value.trim();
    if v.is_empty() {
        return None;
    }
    if !v
        .bytes()
        .all(|b| matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_'))
    {
        return None;
    }
    Some(v)
}
