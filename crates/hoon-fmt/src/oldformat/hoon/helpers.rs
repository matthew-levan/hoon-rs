use indexmap::IndexMap;
use parser::ast::hoon::*;

use super::{format_hoon, format_hoon_wide};
use crate::config::FormatterConfig;
use crate::doc::Doc;
use crate::format::atoms::format_noun_expr;
use crate::format::spec::format_spec;
use crate::format::Format;

pub(super) fn format_axis(n: u64) -> Doc {
    match n {
        0 => Doc::text("+0"),
        1 => Doc::text("."),
        2 => Doc::text("-"),
        3 => Doc::text("+"),
        _ => Doc::text(format!("+{}", n)),
    }
}

pub(super) fn format_leaf(aura: &str, value: &ParsedAtom, config: &FormatterConfig) -> Doc {
    match aura {
        "tas" => {
            let s = atom_to_cord(value);
            Doc::text(format!("%{}", s))
        }
        _ => value.format(config),
    }
}

pub(super) fn format_rock(aura: &str, value: &NounExpr, config: &FormatterConfig) -> Doc {
    match aura {
        "f" => match value {
            NounExpr::ParsedAtom(ParsedAtom::Small(0)) => Doc::text("%.y"),
            NounExpr::ParsedAtom(ParsedAtom::Small(1)) => Doc::text("%.n"),
            _ => format_noun_expr(value, config),
        },
        "n" => Doc::text("~"),
        "tas" => {
            if let NounExpr::ParsedAtom(a) = value {
                let s = atom_to_cord(a);
                Doc::text(format!("%{}", s))
            } else {
                format_noun_expr(value, config)
            }
        }
        _ => format_noun_expr(value, config),
    }
}

pub(super) fn format_sand(aura: &str, value: &NounExpr, config: &FormatterConfig) -> Doc {
    match aura {
        "f" => match value {
            NounExpr::ParsedAtom(ParsedAtom::Small(0)) => Doc::text("&"),
            NounExpr::ParsedAtom(ParsedAtom::Small(1)) => Doc::text("|"),
            _ => format_noun_expr(value, config),
        },
        "t" => {
            if let NounExpr::ParsedAtom(a) = value {
                let s = atom_to_cord(a);
                Doc::text(format!("'{}'", escape_cord(&s)))
            } else {
                format_noun_expr(value, config)
            }
        }
        "ta" => {
            if let NounExpr::ParsedAtom(a) = value {
                let s = atom_to_cord(a);
                Doc::text(format!("~.{}", s))
            } else {
                format_noun_expr(value, config)
            }
        }
        "tas" => {
            if let NounExpr::ParsedAtom(a) = value {
                let s = atom_to_cord(a);
                Doc::text(format!("%{}", s))
            } else {
                format_noun_expr(value, config)
            }
        }
        _ => format_noun_expr(value, config),
    }
}

pub(super) fn format_cord_char(a: &ParsedAtom) -> Doc {
    let c = match a {
        ParsedAtom::Small(n) => (*n & 0xFF) as u8 as char,
        ParsedAtom::Big(b) => {
            let bytes = b.to_bytes_le();
            if bytes.is_empty() {
                return Doc::nil();
            }
            bytes[0] as char
        }
    };
    match c {
        '"' => Doc::text("\\\""),
        '\\' => Doc::text("\\\\"),
        '\n' => Doc::text("\\n"),
        '\t' => Doc::text("\\t"),
        _ => Doc::text(c.to_string()),
    }
}

fn atom_to_cord(value: &ParsedAtom) -> String {
    match value {
        ParsedAtom::Small(n) => {
            let mut s = String::new();
            let mut v = *n;
            while v > 0 {
                s.push((v & 0xFF) as u8 as char);
                v >>= 8;
            }
            s
        }
        ParsedAtom::Big(b) => {
            let bytes = b.to_bytes_le();
            String::from_utf8_lossy(&bytes).to_string()
        }
    }
}

fn escape_cord(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            '\'' => result.push_str("\\'"),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\t' => result.push_str("\\t"),
            _ => result.push(c),
        }
    }
    result
}

pub(super) fn format_core(
    rune: &str,
    name: &Option<String>,
    arms: &IndexMap<String, Tome>,
    config: &FormatterConfig,
) -> Doc {
    let mut parts = vec![Doc::text(rune)];

    if let Some(n) = name {
        parts.push(Doc::gap());
        parts.push(Doc::text(format!("%{}", n)));
    }

    parts.push(format_arms(arms, config));
    parts.push(Doc::hardline());
    parts.push(Doc::text("--"));

    Doc::concat(parts)
}

pub(super) fn format_door(
    rune: &str,
    spec: &Spec,
    alas: &Alas,
    arms: &IndexMap<String, Tome>,
    config: &FormatterConfig,
) -> Doc {
    let mut parts = vec![Doc::text(rune), Doc::gap(), format_spec(spec, config)];

    if !alas.is_empty() {
        let max_name_len = alas.iter().map(|(name, _)| name.len()).max().unwrap_or(0);

        for (i, (name, hoon)) in alas.iter().enumerate() {
            parts.push(Doc::hardline());
            if i == 0 {
                let padding = " ".repeat(max_name_len - name.len());
                parts.push(Doc::text(format!("+*  {}{}", name, padding)));
            } else {
                let padding = " ".repeat(max_name_len - name.len());
                parts.push(Doc::text(format!("    {}{}", name, padding)));
            }
            parts.push(Doc::gap());
            parts.push(format_hoon(hoon, config));
        }
    }

    parts.push(format_arms(arms, config));
    parts.push(Doc::hardline());
    parts.push(Doc::text("--"));

    Doc::concat(parts)
}

pub(super) fn format_arms(arms: &IndexMap<String, Tome>, config: &FormatterConfig) -> Doc {
    let mut parts = Vec::new();

    for (chapter_name, (_what, arms_map)) in arms {
        if !chapter_name.is_empty() && chapter_name != "$" {
            parts.push(Doc::hardline());
            parts.push(Doc::text(format!("|%  {}", chapter_name)));
        }

        for (arm_name, hoon) in arms_map {
            parts.push(Doc::hardline());
            if let Hoon::LusBuc(spec) = hoon {
                let spec_doc = format_spec(spec, config);
                parts.push(Doc::group(Doc::concat(vec![
                    Doc::text(format!("+$  {}", arm_name)),
                    Doc::flat_alt(
                        Doc::nest(
                            config.indent_width as i32,
                            Doc::concat(vec![Doc::hardline(), spec_doc.clone()]),
                        ),
                        Doc::concat(vec![Doc::gap(), spec_doc]),
                    ),
                ])));
            } else {
                parts.push(Doc::text(format!("++  {}", arm_name)));
                parts.push(Doc::nest(
                    config.indent_width as i32,
                    Doc::concat(vec![Doc::hardline(), format_hoon(hoon, config)]),
                ));
            }
        }
    }

    Doc::concat(parts)
}

pub(super) fn format_tyre(tyre: &Tyre, config: &FormatterConfig) -> Doc {
    if tyre.is_empty() {
        return Doc::text("~");
    }

    let pairs: Vec<Doc> = tyre
        .iter()
        .map(|(term, hoon)| {
            Doc::concat(vec![
                Doc::text(format!("%{}", term)),
                Doc::text("  "),
                format_hoon(hoon, config),
            ])
        })
        .collect();

    Doc::concat(vec![
        Doc::text("~["),
        Doc::join(Doc::gap(), pairs),
        Doc::text("]"),
    ])
}

pub(super) fn format_manx(manx: &Manx, config: &FormatterConfig) -> Doc {
    let tag = manx.g.n.format(config);

    let attrs: Vec<Doc> = manx
        .g
        .a
        .iter()
        .map(|(name, beers)| {
            let value = format_beers(beers, config);
            Doc::concat(vec![name.format(config), Doc::text("="), value])
        })
        .collect();

    let attrs_doc = if attrs.is_empty() {
        Doc::nil()
    } else {
        Doc::concat(vec![Doc::text(" "), Doc::join(Doc::text(" "), attrs)])
    };

    let children_doc = format_marl(&manx.c, config);

    if manx.c.is_empty() {
        Doc::concat(vec![Doc::text(";"), tag, attrs_doc, Doc::text(";")])
    } else {
        Doc::concat(vec![
            Doc::text(";"),
            tag,
            attrs_doc,
            Doc::nest(
                config.indent_width as i32,
                Doc::concat(vec![Doc::gap(), children_doc]),
            ),
            Doc::gap(),
            Doc::text("=="),
        ])
    }
}

pub(super) fn format_marl(marl: &Marl, config: &FormatterConfig) -> Doc {
    let parts: Vec<Doc> = marl.iter().map(|t| format_tuna(t, config)).collect();
    Doc::join(Doc::gap(), parts)
}

fn format_tuna(tuna: &Tuna, config: &FormatterConfig) -> Doc {
    match tuna {
        Tuna::Manx(manx) => format_manx(manx, config),
        Tuna::TunaTail(tail) => match tail {
            TunaTail::Tape(hoon) => Doc::concat(vec![
                Doc::text(";\""),
                format_hoon(hoon, config),
                Doc::text("\""),
            ]),
            TunaTail::Manx(hoon) => Doc::concat(vec![Doc::text(";+"), format_hoon(hoon, config)]),
            TunaTail::Marl(hoon) => Doc::concat(vec![Doc::text(";*"), format_hoon(hoon, config)]),
            TunaTail::Call(hoon) => Doc::concat(vec![
                Doc::text(";("),
                format_hoon(hoon, config),
                Doc::text(")"),
            ]),
        },
    }
}

fn format_beers(beers: &[Beer], config: &FormatterConfig) -> Doc {
    let mut parts = vec![Doc::text("\"")];
    for beer in beers {
        match beer {
            Beer::Char(c) => {
                let escaped = match c.as_str() {
                    "\"" => "\\\"",
                    "\\" => "\\\\",
                    "\n" => "\\n",
                    "\t" => "\\t",
                    s => s,
                };
                parts.push(Doc::text(escaped));
            }
            Beer::Hoon(h) => {
                parts.push(Doc::text("{"));
                parts.push(format_hoon_wide(h, config));
                parts.push(Doc::text("}"));
            }
        }
    }
    parts.push(Doc::text("\""));
    Doc::concat(parts)
}
