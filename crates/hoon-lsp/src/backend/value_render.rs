use super::*;
use super::source_lookup::arm_value_for_symbol;

pub(crate) struct ParserValueRender {
    pub(crate) ty: Option<String>,
    pub(crate) value: Option<String>,
}

pub(crate) fn parser_render_value(expr: &str) -> Option<ParserValueRender> {
    parser_render_value_with_scope(expr, None)
}

pub(crate) fn parser_render_value_with_scope(
    expr: &str,
    scope_text: Option<&str>,
) -> Option<ParserValueRender> {
    parser_render_value_with_scope_depth(expr, scope_text, 0)
}

fn parser_render_value_with_scope_depth(
    expr: &str,
    scope_text: Option<&str>,
    depth: usize,
) -> Option<ParserValueRender> {
    if depth > 6 {
        return None;
    }

    let literal = expr.trim();
    if literal.is_empty() {
        return None;
    }

    let parsed = parse_with_metadata(literal, vec!["<hover>".to_string()], true);
    let ast = parsed.ast?;
    let value_node = extract_primary_value_node(&ast);
    let mut render = render_value_node(value_node);
    if render.ty.as_deref() == Some("reference") {
        if let Some(resolved) = resolve_reference_render(value_node, scope_text, depth) {
            render = resolved;
        }
    }
    if let Hoon::BarTis(arg_spec, body) = strip_dbug(value_node) {
        let args = render_gate_args(arg_spec);
        let ret = gate_return_type(arg_spec, body, scope_text, depth)
            .unwrap_or_else(|| "inferred".to_string());
        render.ty = Some("gate".to_string());
        render.value = Some(format!("|= {args} -> {ret}"));
    }
    if matches!(sand_aura(value_node), Some("da" | "dr")) && literal.starts_with('~') {
        render.value = Some(literal.to_string());
    }
    Some(render)
}

fn resolve_reference_render(
    node: &Hoon,
    scope_text: Option<&str>,
    depth: usize,
) -> Option<ParserValueRender> {
    if depth > 6 {
        return None;
    }
    match strip_dbug(node) {
        Hoon::Limb(name) => lookup_symbol_render_in_scope(scope_text, name, depth + 1),
        Hoon::Wing(wing) => wing_last_term(wing)
            .and_then(|name| lookup_symbol_render_in_scope(scope_text, &name, depth + 1)),
        _ => None,
    }
}

fn lookup_symbol_render_in_scope(
    scope_text: Option<&str>,
    symbol: &str,
    depth: usize,
) -> Option<ParserValueRender> {
    if depth > 6 {
        return None;
    }
    let text = scope_text?;
    let expr = arm_value_for_symbol(text, symbol)?;
    parser_render_value_with_scope_depth(&expr, scope_text, depth + 1)
}

fn extract_primary_value_node(node: &Hoon) -> &Hoon {
    let mut current = strip_dbug(node);
    while let Hoon::TisSig(items) = current {
        if items.len() != 1 {
            break;
        }
        current = strip_dbug(&items[0]);
    }
    current
}

fn strip_dbug(mut node: &Hoon) -> &Hoon {
    while let Hoon::Dbug(_, inner) = node {
        node = inner.as_ref();
    }
    node
}

fn render_value_node(node: &Hoon) -> ParserValueRender {
    let node = strip_dbug(node);
    match node {
        Hoon::Bust(BaseType::Null) => ParserValueRender {
            ty: Some("unit".to_string()),
            value: Some("~".to_string()),
        },
        Hoon::Knit(woofs) => {
            let mut out = String::new();
            for woof in woofs {
                if let Woof::ParsedAtom(atom) = woof {
                    if let Some(code) = parsed_atom_to_u32(atom) {
                        if let Some(ch) = char::from_u32(code) {
                            out.push(ch);
                        }
                    }
                }
            }
            ParserValueRender {
                ty: Some("tape".to_string()),
                value: Some(format!("\"{out}\"")),
            }
        }
        Hoon::Sand(aura, noun) => ParserValueRender {
            ty: Some(aura_type_label(aura)),
            value: Some(format_sand_value(aura, noun)),
        },
        Hoon::Rock(aura, noun) => ParserValueRender {
            ty: Some(aura_type_label(aura)),
            value: Some(format!("%{}", noun_expr_to_text(noun))),
        },
        Hoon::BarTis(arg_spec, body) => render_gate_signature(arg_spec, body),
        Hoon::ColSig(items) => render_colsig(items),
        Hoon::Pair(left, right) => {
            let left = render_value_node(left)
                .value
                .unwrap_or_else(|| "<left>".to_string());
            let right = render_value_node(right)
                .value
                .unwrap_or_else(|| "<right>".to_string());
            ParserValueRender {
                ty: Some("cell".to_string()),
                value: Some(format!("[{left} {right}]")),
            }
        }
        Hoon::Wing(wing) => ParserValueRender {
            ty: Some("reference".to_string()),
            value: wing_last_term(wing),
        },
        Hoon::Limb(name) => ParserValueRender {
            ty: Some("reference".to_string()),
            value: Some(name.clone()),
        },
        _ => ParserValueRender {
            ty: Some(node_kind(node).to_string()),
            value: None,
        },
    }
}

fn node_kind(node: &Hoon) -> &'static str {
    match strip_dbug(node) {
        Hoon::Pair(_, _) => "Pair",
        Hoon::Bust(_) => "Bust",
        Hoon::Knit(_) => "Knit",
        Hoon::Limb(_) => "Limb",
        Hoon::Rock(_, _) => "Rock",
        Hoon::Sand(_, _) => "Sand",
        Hoon::Wing(_) => "Wing",
        Hoon::BarTis(_, _) => "BarTis",
        Hoon::ColSig(_) => "ColSig",
        Hoon::ColTar(_) => "ColTar",
        Hoon::TisSig(_) => "TisSig",
        _ => "Hoon",
    }
}

fn render_gate_signature(arg_spec: &Spec, body: &Hoon) -> ParserValueRender {
    let args = render_gate_args(arg_spec);
    let ret = gate_return_type(arg_spec, body, None, 0).unwrap_or_else(|| "inferred".to_string());
    let signature = format!("|= {args} -> {ret}");
    ParserValueRender {
        ty: Some("gate".to_string()),
        value: Some(signature),
    }
}

fn gate_return_type(arg_spec: &Spec, body: &Hoon, scope_text: Option<&str>, depth: usize) -> Option<String> {
    let mut node = strip_dbug(body);
    for _ in 0..8 {
        match node {
            Hoon::KetHep(spec, _) => return Some(render_spec_type(spec)),
            Hoon::KetLus(sample, tail) => {
                if let Some(inferred) = infer_ketlus_return_type(arg_spec, sample, scope_text, depth) {
                    return Some(inferred);
                }
                if let Some(sample_hint) = ketlus_sample_type_hint(sample, scope_text, depth) {
                    return Some(sample_hint);
                }
                node = strip_dbug(tail);
            }
            Hoon::TisSig(items) if !items.is_empty() => {
                node = strip_dbug(&items[0]);
            }
            Hoon::TisCom(_, tail) => {
                node = strip_dbug(tail);
            }
            Hoon::Fits(inner, _)
            | Hoon::Lost(inner)
            | Hoon::BarDot(inner)
            | Hoon::BarHep(inner)
            | Hoon::KetPam(inner)
            | Hoon::KetSig(inner)
            | Hoon::KetWut(inner)
            | Hoon::DotLus(inner)
            | Hoon::DotWut(inner)
            | Hoon::MicFas(inner)
            | Hoon::WutZap(inner)
            | Hoon::ZapGar(inner)
            | Hoon::ZapTis(inner) => {
                node = strip_dbug(inner);
            }
            _ => return None,
        }
    }
    gate_terminal_expression_type(node)
}

fn infer_ketlus_return_type(
    arg_spec: &Spec,
    sample: &Hoon,
    scope_text: Option<&str>,
    depth: usize,
) -> Option<String> {
    match strip_dbug(sample) {
        Hoon::Limb(name) => lookup_gate_arg_type(arg_spec, name)
            .or_else(|| lookup_symbol_type_in_scope(scope_text, name, depth + 1)),
        Hoon::Wing(wing) => wing_last_term(wing).and_then(|name| {
            lookup_gate_arg_type(arg_spec, &name)
                .or_else(|| lookup_symbol_type_in_scope(scope_text, &name, depth + 1))
        }),
        Hoon::Bust(base) => Some(render_base_type(base)),
        Hoon::Sand(aura, _) | Hoon::Rock(aura, _) => {
            if aura.is_empty() {
                Some("@".to_string())
            } else {
                Some(format!("@{aura}"))
            }
        }
        _ => None,
    }
}

fn ketlus_sample_type_hint(sample: &Hoon, scope_text: Option<&str>, depth: usize) -> Option<String> {
    match strip_dbug(sample) {
        Hoon::Limb(name) => lookup_symbol_type_in_scope(scope_text, name, depth + 1),
        Hoon::Wing(wing) => {
            wing_last_term(wing).and_then(|name| lookup_symbol_type_in_scope(scope_text, &name, depth + 1))
        }
        other => {
            let ty = render_value_node(other).ty?;
            if ty == "Hoon" || ty == "reference" {
                None
            } else {
                Some(ty)
            }
        }
    }
}

fn lookup_symbol_type_in_scope(scope_text: Option<&str>, symbol: &str, depth: usize) -> Option<String> {
    if depth > 6 {
        return None;
    }
    let text = scope_text?;
    let expr = arm_value_for_symbol(text, symbol)?;
    let rendered = parser_render_value_with_scope_depth(&expr, scope_text, depth + 1)?;
    let ty = rendered.ty?;
    if ty == "Hoon" || ty == "reference" {
        None
    } else {
        Some(ty)
    }
}

fn lookup_gate_arg_type(spec: &Spec, needle: &str) -> Option<String> {
    for (name, ty) in flatten_gate_arg_bindings(spec) {
        if name == needle {
            return Some(ty);
        }
    }
    None
}

fn flatten_gate_arg_bindings(spec: &Spec) -> Vec<(String, String)> {
    match strip_dbug_spec(spec) {
        Spec::BucCol(head, tail) => {
            let mut out = flatten_gate_arg_bindings(head);
            for item in tail {
                out.extend(flatten_gate_arg_bindings(item));
            }
            out
        }
        Spec::BucTis(skin, inner) => match skin {
            Skin::Term(name) => vec![(name.clone(), render_spec_type(inner))],
            _ => Vec::new(),
        },
        Spec::Name(name, inner) => vec![(name.clone(), render_spec_type(inner))],
        _ => Vec::new(),
    }
}

fn gate_terminal_expression_type(node: &Hoon) -> Option<String> {
    let mut cur = strip_dbug(node);
    for _ in 0..8 {
        match cur {
            Hoon::TisSig(items) if !items.is_empty() => {
                cur = strip_dbug(items.last().expect("non-empty checked"));
            }
            Hoon::TisCom(_, tail) => {
                cur = strip_dbug(tail);
            }
            Hoon::KetHep(_, tail) | Hoon::KetLus(_, tail) => {
                cur = strip_dbug(tail);
            }
            Hoon::Fits(inner, _)
            | Hoon::Lost(inner)
            | Hoon::BarDot(inner)
            | Hoon::BarHep(inner)
            | Hoon::KetPam(inner)
            | Hoon::KetSig(inner)
            | Hoon::KetWut(inner)
            | Hoon::DotLus(inner)
            | Hoon::DotWut(inner)
            | Hoon::MicFas(inner)
            | Hoon::WutZap(inner)
            | Hoon::ZapGar(inner)
            | Hoon::ZapTis(inner) => {
                cur = strip_dbug(inner);
            }
            _ => break,
        }
    }

    let ty = render_value_node(cur).ty?;
    if ty == "Hoon" || ty == "reference" {
        None
    } else {
        Some(ty)
    }
}

fn render_gate_args(spec: &Spec) -> String {
    let flat = flatten_gate_args(spec);
    if flat.is_empty() {
        render_spec_type(spec)
    } else if flat.len() == 1 {
        flat[0].clone()
    } else {
        format!("[{}]", flat.join(" "))
    }
}

fn flatten_gate_args(spec: &Spec) -> Vec<String> {
    match strip_dbug_spec(spec) {
        Spec::BucCol(head, tail) => {
            let mut out = flatten_gate_args(head);
            for item in tail {
                out.extend(flatten_gate_args(item));
            }
            out
        }
        Spec::BucTis(skin, inner) => {
            let name = render_skin_name(skin);
            let ty = render_spec_type(inner);
            vec![format!("{name}={ty}")]
        }
        _ => Vec::new(),
    }
}

fn render_skin_name(skin: &Skin) -> String {
    match skin {
        Skin::Term(name) => name.clone(),
        _ => "_".to_string(),
    }
}

fn strip_dbug_spec(mut spec: &Spec) -> &Spec {
    while let Spec::Dbug(_, inner) = spec {
        spec = inner.as_ref();
    }
    spec
}

fn render_spec_type(spec: &Spec) -> String {
    match strip_dbug_spec(spec) {
        Spec::Base(base) => render_base_type(base),
        Spec::Like(wing, wings) => {
            let mut out = render_wing_type(wing);
            if !wings.is_empty() {
                let tail = wings
                    .iter()
                    .map(|w| render_wing_type(w))
                    .collect::<Vec<_>>()
                    .join(",");
                out.push(':');
                out.push_str(&tail);
            }
            out
        }
        Spec::Name(name, inner) => format!("{name}:{}", render_spec_type(inner)),
        Spec::BucTis(skin, inner) => format!("{}={}", render_skin_name(skin), render_spec_type(inner)),
        Spec::BucCol(head, tail) => {
            let mut items = vec![render_spec_type(head)];
            items.extend(tail.iter().map(render_spec_type));
            format!("[{}]", items.join(" "))
        }
        Spec::BucCen(head, tail) => {
            let mut items = vec![render_spec_type(head)];
            items.extend(tail.iter().map(render_spec_type));
            format!("$: {} ==", items.join(" "))
        }
        _ => "spec".to_string(),
    }
}

fn render_wing_type(wing: &[Limb]) -> String {
    let mut out = Vec::new();
    for limb in wing {
        match limb {
            Limb::Term(term) => out.push(term.clone()),
            Limb::Axis(axis) => out.push(axis.to_string()),
            Limb::Parent(axis, maybe_term) => {
                if let Some(term) = maybe_term {
                    out.push(format!("{term}:{axis}"));
                } else {
                    out.push(axis.to_string());
                }
            }
        }
    }
    if out.is_empty() {
        "*".to_string()
    } else {
        out.join(":")
    }
}

fn render_base_type(base: &BaseType) -> String {
    match base {
        BaseType::NounExpr => "*".to_string(),
        BaseType::Cell => "cell".to_string(),
        BaseType::Flag => "?".to_string(),
        BaseType::Null => "~".to_string(),
        BaseType::Void => "~|".to_string(),
        BaseType::Atom(aura) => {
            if aura.is_empty() {
                "@".to_string()
            } else {
                format!("@{aura}")
            }
        }
    }
}

fn sand_aura(node: &Hoon) -> Option<&str> {
    match strip_dbug(node) {
        Hoon::Sand(aura, _) => Some(aura.as_str()),
        _ => None,
    }
}

fn parsed_atom_to_u32(atom: &ParsedAtom) -> Option<u32> {
    atom.to_u32()
}

fn parsed_atom_to_string(atom: &ParsedAtom) -> String {
    match atom {
        ParsedAtom::Small(v) => v.to_string(),
        ParsedAtom::Big(v) => v.to_string(),
    }
}

fn parsed_atom_to_text(atom: &ParsedAtom) -> String {
    let bytes = atom.to_biguint().to_bytes_le();
    if bytes.is_empty() {
        return String::new();
    }
    String::from_utf8(bytes).unwrap_or_else(|_| parsed_atom_to_string(atom))
}

fn noun_expr_to_text(noun: &NounExpr) -> String {
    match noun {
        NounExpr::ParsedAtom(atom) => parsed_atom_to_text(atom),
        NounExpr::Cell(head, tail) => {
            let head = noun_expr_to_text(head);
            let tail = noun_expr_to_text(tail);
            format!("[{head} {tail}]")
        }
    }
}

fn noun_expr_to_decimal(noun: &NounExpr) -> String {
    match noun {
        NounExpr::ParsedAtom(atom) => parsed_atom_to_string(atom),
        NounExpr::Cell(head, tail) => {
            let head = noun_expr_to_decimal(head);
            let tail = noun_expr_to_decimal(tail);
            format!("[{head} {tail}]")
        }
    }
}

fn wing_last_term(wing: &[Limb]) -> Option<String> {
    for limb in wing.iter().rev() {
        if let Limb::Term(term) = limb {
            return Some(term.clone());
        }
    }
    None
}

fn aura_type_label(aura: &str) -> String {
    let (base, suffix) = split_aura_and_bitwidth(aura);
    let label = match base {
        "" => "empty aura (@)",
        "c" => "UTF-32 (@c)",
        "d" => "date (@d)",
        "da" => "absolute date (@da)",
        "dr" => "relative date/timespan (@dr)",
        "f" => "Loobean, compiler-only (@f)",
        "i" => "internet address (@i)",
        "if" => "IPv4 address (@if)",
        "is" => "IPv6 address (@is)",
        "n" => "nil, compiler-only (@n)",
        "p" => "phonemic base (@p)",
        "q" => "phonemic base unscrambled (@q)",
        "r" => "floating-point (@r)",
        "rh" => "half precision float (@rh)",
        "rs" => "single precision float (@rs)",
        "rd" => "double precision float (@rd)",
        "rq" => "quad precision float (@rq)",
        "s" => "signed integer (@s)",
        "sb" => "signed binary (@sb)",
        "sd" => "signed decimal (@sd)",
        "si" => "signed decimal no separator (@si)",
        "sv" => "signed base32 (@sv)",
        "sw" => "signed base64 (@sw)",
        "sx" => "signed hexadecimal (@sx)",
        "t" => "cord (@t)",
        "ta" => "knot (@ta)",
        "tas" => "term (@tas)",
        "u" => "unsigned integer (@u)",
        "ub" => "unsigned binary (@ub)",
        "ud" => "unsigned decimal (@ud)",
        "ui" => "unsigned decimal no separator (@ui)",
        "uv" => "unsigned base32 (@uv)",
        "uw" => "unsigned base64 (@uw)",
        "ux" => "unsigned hexadecimal (@ux)",
        "x" => "unsigned hexadecimal, no separator (@x)",
        _ => {
            if aura.is_empty() {
                "empty aura (@)"
            } else {
                return format!("@{aura}");
            }
        }
    };

    if let Some(width) = suffix {
        format!("{label}, bitwidth {width}")
    } else {
        label.to_string()
    }
}

fn render_colsig(items: &[Hoon]) -> ParserValueRender {
    let element_type = items
        .first()
        .map(list_element_type)
        .unwrap_or_else(|| "noun".to_string());

    let mut rendered_items = Vec::new();
    for item in items.iter().take(16) {
        let rendered = render_value_node(item)
            .value
            .unwrap_or_else(|| "<item>".to_string());
        rendered_items.push(rendered);
    }
    if items.len() > 16 {
        rendered_items.push("...".to_string());
    }

    let value = if rendered_items.is_empty() {
        "~".to_string()
    } else {
        format!("~[{}]", rendered_items.join(" "))
    };

    ParserValueRender {
        ty: Some(format!("(list {element_type})")),
        value: Some(value),
    }
}

fn list_element_type(item: &Hoon) -> String {
    match strip_dbug(item) {
        Hoon::Sand(aura, _) | Hoon::Rock(aura, _) => {
            if aura.is_empty() {
                "@".to_string()
            } else {
                format!("@{aura}")
            }
        }
        _ => render_value_node(item)
            .ty
            .unwrap_or_else(|| "noun".to_string()),
    }
}

fn split_aura_and_bitwidth(aura: &str) -> (&str, Option<char>) {
    let mut chars = aura.chars();
    let Some(last) = chars.next_back() else {
        return ("", None);
    };
    if last.is_ascii_uppercase() {
        let base_len = aura.len().saturating_sub(last.len_utf8());
        (&aura[..base_len], Some(last))
    } else {
        (aura, None)
    }
}

fn format_sand_value(aura: &str, atom: &NounExpr) -> String {
    let (base, _) = split_aura_and_bitwidth(aura);
    match base {
        "t" | "ta" | "tas" => format!("'{}'", noun_expr_to_text(atom)),
        _ => noun_expr_to_decimal(atom),
    }
}
