//! Formatting for Spec (type specification) AST nodes.

use crate::config::FormatterConfig;
use crate::doc::Doc;
use crate::format::atoms::format_wing;
use crate::format::hoon::format_hoon;
use crate::format::runes::*;
use crate::format::{rune2_backstep, rune_vararg, tall_or_wide, Format};
use parser::ast::hoon::*;

/// Format a Spec to a Doc.
pub fn format_spec(spec: &Spec, config: &FormatterConfig) -> Doc {
    match spec {
        Spec::Base(base) => base.format(config),

        Spec::Dbug(_spot, inner) => {
            // Skip debug wrapper, just format inner
            format_spec(inner, config)
        }

        Spec::Leaf(aura, value) => {
            // Constant type like %foo
            if aura == "tas" {
                // Term constant
                let s = atom_to_cord(value);
                Doc::text(format!("%{}", s))
            } else {
                Doc::concat(vec![Doc::text("%"), value.format(config)])
            }
        }

        Spec::Like(wing, args) => {
            // Type reference like foo or foo:bar
            if args.is_empty() {
                format_wing(wing, config)
            } else {
                let mut parts = vec![format_wing(wing, config)];
                for arg in args {
                    parts.push(Doc::text(":"));
                    parts.push(format_wing(arg, config));
                }
                Doc::concat(parts)
            }
        }

        Spec::Loop(name) => {
            // Recursive type reference
            Doc::text(format!("${}", name))
        }

        Spec::Made((name, _args), inner) => {
            // Named type with possible type args
            Doc::concat(vec![
                Doc::text(format!("%{}.", name)),
                format_spec(inner, config),
            ])
        }

        Spec::Make(hoon, args) => {
            // Constructed type
            let hoon_doc = format_hoon(hoon, config);
            if args.is_empty() {
                Doc::concat(vec![Doc::text("("), hoon_doc, Doc::text(")")])
            } else {
                let arg_docs: Vec<Doc> = args.iter().map(|a| format_spec(a, config)).collect();
                Doc::concat(vec![
                    Doc::text("("),
                    hoon_doc,
                    Doc::text(" "),
                    Doc::join(Doc::text(" "), arg_docs),
                    Doc::text(")"),
                ])
            }
        }

        Spec::Name(name, inner) => {
            // Named type binding: name=spec
            Doc::concat(vec![
                Doc::text(name.clone()),
                Doc::text("="),
                format_spec(inner, config),
            ])
        }

        Spec::Over(wing, inner) => {
            // Type derived from wing
            Doc::concat(vec![
                format_wing(wing, config),
                Doc::text("/"),
                format_spec(inner, config),
            ])
        }

        // $> bucgar - filter by head
        Spec::BucGar(head, tail) => {
            tall_or_wide(
                BUC_GAR,
                vec![format_spec(head, config), format_spec(tail, config)],
            )
        }

        // $$ bucbuc - recursive structure
        Spec::BucBuc(inner, arms) => {
            let inner_doc = format_spec(inner, config);
            let arm_docs: Vec<Doc> = arms
                .iter()
                .map(|(name, spec)| {
                    Doc::concat(vec![
                        Doc::text(format!("++  {}", name)),
                        Doc::gap(),
                        format_spec(spec, config),
                    ])
                })
                .collect();
            Doc::concat(vec![
                Doc::text(BUC_BUC),
                Doc::gap(),
                inner_doc,
                Doc::concat(arm_docs),
                Doc::gap(),
                Doc::text("--"),
            ])
        }

        // $| bucbar - normalizing gate
        Spec::BucBar(spec, gate) => {
            tall_or_wide(
                BUC_BAR,
                vec![format_spec(spec, config), format_hoon(gate, config)],
            )
        }

        // $_ buccab - example type
        Spec::BucCab(example) => {
            Doc::concat(vec![Doc::text(BUC_CAB), Doc::gap(), format_hoon(example, config)])
        }

        // $: buccol - tuple type (tall mode with line breaks)
        // Children indented 2 spaces relative to $:
        Spec::BucCol(head, tail) => {
            let mut child_parts = vec![];
            // Add head
            child_parts.push(Doc::hardline());
            child_parts.push(format_spec(head, config));
            // Add tail elements
            for spec in tail {
                child_parts.push(Doc::hardline());
                child_parts.push(format_spec(spec, config));
            }
            // Nest the entire $: block so children are indented relative to rune position
            Doc::nest(
                config.indent_width as i32,
                Doc::concat(vec![
                    Doc::text(BUC_COL),
                    Doc::nest(config.indent_width as i32, Doc::concat(child_parts)),
                    Doc::hardline(),
                    Doc::text("=="),
                ]),
            )
        }

        // $% buccen - tagged union
        Spec::BucCen(head, cases) => {
            let mut children = vec![format_spec(head, config)];
            children.extend(cases.iter().map(|s| format_spec(s, config)));
            rune_vararg(BUC_CEN, children, config.indent_width as i32)
        }

        // $. bucdot - cell with sample
        Spec::BucDot(spec, arms) => {
            let spec_doc = format_spec(spec, config);
            let arm_docs: Vec<Doc> = arms
                .iter()
                .map(|(name, s)| {
                    Doc::concat(vec![
                        Doc::text(format!("{}=", name)),
                        format_spec(s, config),
                    ])
                })
                .collect();
            Doc::concat(vec![
                Doc::text(BUC_DOT),
                Doc::gap(),
                spec_doc,
                Doc::gap(),
                Doc::join(Doc::gap(), arm_docs),
            ])
        }

        // $< bucgal - filter by tail
        Spec::BucGal(head, tail) => {
            tall_or_wide(
                BUC_GAL,
                vec![format_spec(head, config), format_spec(tail, config)],
            )
        }

        // $- buchep - gate type
        Spec::BucHep(arg, ret) => {
            tall_or_wide(
                BUC_HEP,
                vec![format_spec(arg, config), format_spec(ret, config)],
            )
        }

        // $^ bucket - cell of types
        Spec::BucKet(head, tail) => {
            tall_or_wide(
                BUC_KET,
                vec![format_spec(head, config), format_spec(tail, config)],
            )
        }

        // $+ buclus - alias
        Spec::BucLus(name, inner) => {
            rune2_backstep(
                BUC_LUS,
                Doc::text(name.clone()),
                format_spec(inner, config),
                config.indent_width as i32,
            )
        }

        // $/ bucfas - mold builder
        Spec::BucFas(spec, arms) => {
            let spec_doc = format_spec(spec, config);
            let arm_docs: Vec<Doc> = arms
                .iter()
                .map(|(name, s)| {
                    Doc::concat(vec![
                        Doc::text(format!("{}=", name)),
                        format_spec(s, config),
                    ])
                })
                .collect();
            Doc::concat(vec![
                Doc::text(BUC_FAS),
                Doc::gap(),
                spec_doc,
                Doc::gap(),
                Doc::join(Doc::gap(), arm_docs),
            ])
        }

        // $; bucmic - manual normalizer
        Spec::BucMic(gate) => {
            Doc::concat(vec![Doc::text(BUC_MIC), Doc::gap(), format_hoon(gate, config)])
        }

        // $& bucpam - pair test
        Spec::BucPam(spec, test) => {
            tall_or_wide(
                BUC_PAM,
                vec![format_spec(spec, config), format_hoon(test, config)],
            )
        }

        // $~ bucsig - default value
        Spec::BucSig(default, inner) => {
            tall_or_wide(
                BUC_SIG,
                vec![format_hoon(default, config), format_spec(inner, config)],
            )
        }

        // $` buctis - face with wrap
        Spec::BucTic(spec, arms) => {
            let spec_doc = format_spec(spec, config);
            let arm_docs: Vec<Doc> = arms
                .iter()
                .map(|(name, s)| {
                    Doc::concat(vec![
                        Doc::text(format!("{}=", name)),
                        format_spec(s, config),
                    ])
                })
                .collect();
            Doc::concat(vec![
                Doc::text(BUC_TIC),
                Doc::gap(),
                spec_doc,
                Doc::gap(),
                Doc::join(Doc::gap(), arm_docs),
            ])
        }

        // $= buctis - face binding (prefer tall/gap mode for type declarations)
        Spec::BucTis(skin, inner) => {
            Doc::concat(vec![
                Doc::text(BUC_TIS),
                Doc::gap(),
                format_skin(skin, config),
                Doc::gap(),
                format_spec(inner, config),
            ])
        }

        // $@ bucpat - atom/cell test
        Spec::BucPat(atom_case, cell_case) => {
            tall_or_wide(
                BUC_PAT,
                vec![
                    format_spec(atom_case, config),
                    format_spec(cell_case, config),
                ],
            )
        }

        // $? bucwut - union type
        Spec::BucWut(head, cases) => {
            let mut children = vec![format_spec(head, config)];
            children.extend(cases.iter().map(|s| format_spec(s, config)));
            rune_vararg(BUC_WUT, children, config.indent_width as i32)
        }

        // $! buczap - mold test
        Spec::BucZap(spec, arms) => {
            let spec_doc = format_spec(spec, config);
            let arm_docs: Vec<Doc> = arms
                .iter()
                .map(|(name, s)| {
                    Doc::concat(vec![
                        Doc::text(format!("{}=", name)),
                        format_spec(s, config),
                    ])
                })
                .collect();
            Doc::concat(vec![
                Doc::text(BUC_ZAP),
                Doc::gap(),
                spec_doc,
                Doc::gap(),
                Doc::join(Doc::gap(), arm_docs),
            ])
        }

        // Preserved irregular syntax: [a=@ b=tape]
        Spec::CellLiteral(specs) => {
            let children: Vec<Doc> = specs.iter().map(|s| format_spec(s, config)).collect();
            Doc::concat(vec![
                Doc::text("["),
                Doc::join(Doc::text(" "), children),
                Doc::text("]"),
            ])
        }

        // Preserved irregular syntax: a=@
        Spec::NameBinding(name, inner) => {
            Doc::concat(vec![
                Doc::text(name.clone()),
                Doc::text("="),
                format_spec(inner, config),
            ])
        }
    }
}

/// Format a Skin (type binding pattern).
pub fn format_skin(skin: &Skin, config: &FormatterConfig) -> Doc {
    match skin {
        Skin::Term(name) => Doc::text(name.clone()),
        Skin::Base(base) => base.format(config),
        Skin::Cell(head, tail) => {
            Doc::concat(vec![
                Doc::text("["),
                format_skin(head, config),
                Doc::text(" "),
                format_skin(tail, config),
                Doc::text("]"),
            ])
        }
        Skin::Dbug(_spot, inner) => format_skin(inner, config),
        Skin::Leaf(aura, value) => {
            if aura == "tas" {
                let s = atom_to_cord(value);
                Doc::text(format!("%{}", s))
            } else {
                Doc::concat(vec![Doc::text("%"), value.format(config)])
            }
        }
        Skin::Name(name, inner) => {
            Doc::concat(vec![
                Doc::text(name.clone()),
                Doc::text("="),
                format_skin(inner, config),
            ])
        }
        Skin::Over(wing, inner) => {
            Doc::concat(vec![
                format_wing(wing, config),
                Doc::text("/"),
                format_skin(inner, config),
            ])
        }
        Skin::Spec(spec, inner) => {
            Doc::concat(vec![
                format_spec(spec, config),
                Doc::text("/"),
                format_skin(inner, config),
            ])
        }
        Skin::Wash(n) => Doc::text(format!("_{}", n)),
    }
}

/// Convert an atom to a cord string.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::Renderer;

    #[test]
    fn test_base_type() {
        let config = FormatterConfig::default();
        let renderer = Renderer::new(config.clone());

        assert_eq!(
            renderer.render(&format_spec(&Spec::Base(BaseType::NounExpr), &config)),
            "*"
        );
        assert_eq!(
            renderer.render(&format_spec(&Spec::Base(BaseType::Flag), &config)),
            "?"
        );
        assert_eq!(
            renderer.render(&format_spec(&Spec::Base(BaseType::Null), &config)),
            "~"
        );
    }
}
