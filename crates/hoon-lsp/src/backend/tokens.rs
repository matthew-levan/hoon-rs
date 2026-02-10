pub(super) fn token_at_position(text: &str, byte_offset: usize) -> Option<String> {
    if text.is_empty() {
        return None;
    }

    let bytes = text.as_bytes();
    let mut idx = byte_offset.min(text.len());
    if idx == text.len() && idx > 0 {
        idx -= 1;
    }

    if is_token_boundary(bytes.get(idx).copied()) {
        return None;
    }

    let mut start = idx;
    while start > 0 && !is_token_boundary(bytes.get(start - 1).copied()) {
        start -= 1;
    }

    let mut end = idx + 1;
    while end < text.len() && !is_token_boundary(bytes.get(end).copied()) {
        end += 1;
    }

    let token = text.get(start..end)?.trim();
    if token.is_empty() {
        return None;
    }

    Some(token.to_string())
}

pub(super) fn namespace_rhs_symbol_at_position(text: &str, byte_offset: usize) -> Option<String> {
    if text.is_empty() {
        return None;
    }

    let bytes = text.as_bytes();
    let mut idx = byte_offset.min(text.len());
    if idx == text.len() && idx > 0 {
        idx -= 1;
    }

    if bytes.get(idx).copied() == Some(b':') {
        let rhs_start = idx + 1;
        if rhs_start >= text.len() || !is_symbol_ident_byte(bytes.get(rhs_start).copied()) {
            return None;
        }
        let mut rhs_end = rhs_start;
        while rhs_end < text.len() && is_symbol_ident_byte(bytes.get(rhs_end).copied()) {
            rhs_end += 1;
        }
        return text.get(rhs_start..rhs_end).map(ToOwned::to_owned);
    }

    if !is_symbol_ident_byte(bytes.get(idx).copied()) {
        return None;
    }

    let mut start = idx;
    while start > 0 && is_symbol_ident_byte(bytes.get(start - 1).copied()) {
        start -= 1;
    }
    if start == 0 || bytes.get(start - 1).copied() != Some(b':') {
        return None;
    }

    let mut end = idx + 1;
    while end < text.len() && is_symbol_ident_byte(bytes.get(end).copied()) {
        end += 1;
    }
    text.get(start..end).map(ToOwned::to_owned)
}

pub(super) fn namespace_member_symbol_at_position(text: &str, byte_offset: usize) -> Option<(String, String)> {
    if text.is_empty() {
        return None;
    }

    let bytes = text.as_bytes();
    let mut idx = byte_offset.min(text.len());
    if idx == text.len() && idx > 0 {
        idx -= 1;
    }
    if !is_symbol_ident_byte(bytes.get(idx).copied()) {
        return None;
    }

    let mut start = idx;
    while start > 0 && is_symbol_ident_byte(bytes.get(start - 1).copied()) {
        start -= 1;
    }
    let mut end = idx + 1;
    while end < text.len() && is_symbol_ident_byte(bytes.get(end).copied()) {
        end += 1;
    }

    if bytes.get(end).copied() != Some(b':') {
        return None;
    }
    let module_start = end + 1;
    if module_start >= text.len() || !is_symbol_ident_byte(bytes.get(module_start).copied()) {
        return None;
    }
    let mut module_end = module_start;
    while module_end < text.len() && is_symbol_ident_byte(bytes.get(module_end).copied()) {
        module_end += 1;
    }

    let member = text.get(start..end)?.to_string();
    let module = text.get(module_start..module_end)?.to_string();
    Some((member, module))
}

fn is_symbol_ident_byte(b: Option<u8>) -> bool {
    matches!(b, Some(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_'))
}

pub(super) fn is_simple_symbol_name(symbol: &str) -> bool {
    !symbol.is_empty()
        && symbol
            .bytes()
            .all(|b| matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_'))
}

fn is_token_boundary(b: Option<u8>) -> bool {
    match b {
        None => true,
        Some(c) if c.is_ascii_whitespace() => true,
        Some(b'(' | b')' | b'[' | b']' | b'{' | b'}' | b',' | b';') => true,
        _ => false,
    }
}
