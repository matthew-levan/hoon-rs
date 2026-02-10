//! Formatting for atoms, limbs, wings, and other basic elements.

use crate::config::FormatterConfig;
use crate::doc::Doc;
use crate::format::Format;
use num_bigint::BigUint;
use parser::ast::hoon::*;

impl Format for ParsedAtom {
    fn format(&self, _config: &FormatterConfig) -> Doc {
        match self {
            ParsedAtom::Small(n) => Doc::text(format_decimal(*n)),
            ParsedAtom::Big(n) => Doc::text(format_biguint(n)),
        }
    }
}

/// Format a decimal number with Hoon's dot separators (every 3 digits).
fn format_decimal(n: u128) -> String {
    if n < 1000 {
        return n.to_string();
    }

    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();

    for (i, c) in chars.iter().enumerate() {
        result.push(*c);
        let remaining = len - i - 1;
        if remaining > 0 && remaining % 3 == 0 {
            result.push('.');
        }
    }
    result
}

/// Format a BigUint with dot separators.
fn format_biguint(n: &BigUint) -> String {
    let s = n.to_string();
    if s.len() < 4 {
        return s;
    }

    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();

    for (i, c) in chars.iter().enumerate() {
        result.push(*c);
        let remaining = len - i - 1;
        if remaining > 0 && remaining % 3 == 0 {
            result.push('.');
        }
    }
    result
}

impl Format for Limb {
    fn format(&self, _config: &FormatterConfig) -> Doc {
        match self {
            Limb::Term(name) => Doc::text(name.clone()),
            Limb::Axis(n) => {
                // Format axis as +n or use special cases
                match n {
                    1 => Doc::text("."),
                    2 => Doc::text("-"),
                    3 => Doc::text("+"),
                    _ => Doc::text(format!("+{}", n)),
                }
            }
            Limb::Parent(count, name) => {
                let mut s = "^".repeat(*count as usize);
                if let Some(n) = name {
                    s.push_str(n);
                }
                Doc::text(s)
            }
        }
    }
}

/// Format a wing (list of limbs separated by dots).
pub fn format_wing(wing: &WingType, config: &FormatterConfig) -> Doc {
    if wing.is_empty() {
        return Doc::nil();
    }
    if wing.len() == 1 {
        return wing[0].format(config);
    }

    let parts: Vec<Doc> = wing.iter().map(|l| l.format(config)).collect();
    Doc::join(Doc::text("."), parts)
}

impl Format for BaseType {
    fn format(&self, _config: &FormatterConfig) -> Doc {
        match self {
            BaseType::NounExpr => Doc::text("*"),
            BaseType::Cell => Doc::text("^"),
            BaseType::Flag => Doc::text("?"),
            BaseType::Null => Doc::text("~"),
            BaseType::Void => Doc::text("!!"),
            BaseType::Atom(aura) => {
                if aura.is_empty() {
                    Doc::text("@")
                } else {
                    Doc::text(format!("@{}", aura))
                }
            }
        }
    }
}

impl Format for Coin {
    fn format(&self, config: &FormatterConfig) -> Doc {
        match self {
            Coin::Dime(aura, value) => {
                format_dime(aura, value, config)
            }
            Coin::Blob(noun) => {
                // Format as ~noun
                Doc::concat(vec![Doc::text("~"), format_noun_expr(noun, config)])
            }
            Coin::Many(coins) => {
                let parts: Vec<Doc> = coins.iter().map(|c| c.format(config)).collect();
                Doc::join(Doc::text("."), parts)
            }
        }
    }
}

/// Format a dime (aura + value).
fn format_dime(aura: &str, value: &ParsedAtom, config: &FormatterConfig) -> Doc {
    match aura {
        // Unsigned decimal
        "ud" => value.format(config),
        // Unsigned hexadecimal
        "ux" => format_hex(value),
        // Unsigned binary
        "ub" => format_binary(value),
        // Cord (text)
        "t" => format_cord(value),
        // Term
        "tas" => format_term(value),
        // Ship name
        "p" => format_ship(value),
        // Date
        "da" => format_date(value),
        // Timespan
        "dr" => format_timespan(value),
        // Signed integer
        "sd" => format_signed(value),
        // Floating point
        "rs" | "rd" | "rq" | "rh" => format_float(aura, value),
        // Other auras - just use decimal with aura prefix
        _ => {
            if aura.is_empty() {
                value.format(config)
            } else {
                // Format value first, then concatenate
                let value_str = match value {
                    ParsedAtom::Small(n) => format_decimal(*n),
                    ParsedAtom::Big(n) => format_biguint(n),
                };
                Doc::text(format!("0{}.{}", aura, value_str))
            }
        }
    }
}

fn format_hex(value: &ParsedAtom) -> Doc {
    let n = match value {
        ParsedAtom::Small(n) => *n as u64,
        ParsedAtom::Big(b) => {
            // For now, just show large hex numbers
            return Doc::text(format!("0x{:x}", b));
        }
    };
    Doc::text(format!("0x{:x}", n))
}

fn format_binary(value: &ParsedAtom) -> Doc {
    let n = match value {
        ParsedAtom::Small(n) => *n as u64,
        ParsedAtom::Big(b) => {
            return Doc::text(format!("0b{:b}", b));
        }
    };
    Doc::text(format!("0b{:b}", n))
}

fn format_cord(value: &ParsedAtom) -> Doc {
    // Convert atom to string
    let s = atom_to_string(value);
    Doc::text(format!("'{}'", escape_cord(&s)))
}

fn format_term(value: &ParsedAtom) -> Doc {
    let s = atom_to_string(value);
    Doc::text(format!("%{}", s))
}

fn format_ship(_value: &ParsedAtom) -> Doc {
    // TODO: Proper @p formatting (phonemic encoding)
    Doc::text("~zod") // Placeholder
}

fn format_date(_value: &ParsedAtom) -> Doc {
    // TODO: Proper date formatting
    Doc::text("~2000.1.1") // Placeholder
}

fn format_timespan(_value: &ParsedAtom) -> Doc {
    // TODO: Proper timespan formatting
    Doc::text("~s0") // Placeholder
}

fn format_signed(value: &ParsedAtom) -> Doc {
    // Signed integers use zigzag encoding
    let n = match value {
        ParsedAtom::Small(n) => *n as i128,
        ParsedAtom::Big(_) => return Doc::text("--0"), // Placeholder for big signed
    };
    if n >= 0 {
        Doc::text(format!("--{}", n))
    } else {
        Doc::text(format!("-{}", -n))
    }
}

fn format_float(_aura: &str, _value: &ParsedAtom) -> Doc {
    // TODO: Proper float formatting
    Doc::text(".0") // Placeholder
}

/// Convert an atom to a string (for cords/terms).
fn atom_to_string(value: &ParsedAtom) -> String {
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

/// Escape a cord for output.
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

/// Format a NounExpr.
pub fn format_noun_expr(expr: &NounExpr, config: &FormatterConfig) -> Doc {
    match expr {
        NounExpr::ParsedAtom(a) => a.format(config),
        NounExpr::Cell(left, right) => {
            Doc::concat(vec![
                Doc::text("["),
                format_noun_expr(left, config),
                Doc::text(" "),
                format_noun_expr(right, config),
                Doc::text("]"),
            ])
        }
    }
}

impl Format for Mane {
    fn format(&self, _config: &FormatterConfig) -> Doc {
        match self {
            Mane::Tag(s) => Doc::text(s.clone()),
            Mane::TagSpace(ns, tag) => Doc::text(format!("{}:{}", ns, tag)),
        }
    }
}

impl Format for Chum {
    fn format(&self, _config: &FormatterConfig) -> Doc {
        match self {
            Chum::Lef(s) => Doc::text(format!("%{}", s)),
            Chum::StdKel(s, n) => {
                Doc::text(format!("%{}.{}", s, format_decimal(n.to_u128().unwrap_or(0))))
            }
            Chum::VenProKel(vendor, product, version) => {
                Doc::text(format!(
                    "%{}.{}.{}",
                    vendor,
                    product,
                    format_decimal(version.to_u128().unwrap_or(0))
                ))
            }
            Chum::VenProVerKel(vendor, product, version, kelvin) => {
                Doc::text(format!(
                    "%{}.{}.{}.{}",
                    vendor,
                    product,
                    format_decimal(version.to_u128().unwrap_or(0)),
                    format_decimal(kelvin.to_u128().unwrap_or(0))
                ))
            }
        }
    }
}

impl Format for Note {
    fn format(&self, config: &FormatterConfig) -> Doc {
        match self {
            Note::Know(s) => Doc::text(format!("%{}", s)),
            Note::Made(s, wings) => {
                let mut parts = vec![Doc::text(format!("%{}", s))];
                if let Some(ws) = wings {
                    for w in ws {
                        parts.push(Doc::text(" "));
                        parts.push(format_wing(w, config));
                    }
                }
                Doc::concat(parts)
            }
        }
    }
}

impl Format for TermOrPair {
    fn format(&self, config: &FormatterConfig) -> Doc {
        match self {
            TermOrPair::Term(s) => Doc::text(format!("%{}", s)),
            TermOrPair::Pair(s, hoon) => {
                use crate::format::hoon::format_hoon;
                Doc::concat(vec![
                    Doc::text(format!("[%{} ", s)),
                    format_hoon(hoon, config),
                    Doc::text("]"),
                ])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_decimal() {
        assert_eq!(format_decimal(0), "0");
        assert_eq!(format_decimal(123), "123");
        assert_eq!(format_decimal(1234), "1.234");
        assert_eq!(format_decimal(1234567), "1.234.567");
    }

    #[test]
    fn test_limb_format() {
        let config = FormatterConfig::default();
        assert_eq!(
            crate::render::Renderer::new(config.clone())
                .render(&Limb::Term("foo".to_string()).format(&config)),
            "foo"
        );
        assert_eq!(
            crate::render::Renderer::new(config.clone())
                .render(&Limb::Axis(1).format(&config)),
            "."
        );
    }
}
