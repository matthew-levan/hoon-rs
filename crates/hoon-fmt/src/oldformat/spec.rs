//! Formatting for Spec (type specification) AST nodes.
//!
//! Uses FormatMode to enforce strict tall/wide separation.

use crate::config::FormatterConfig;
use crate::doc::Doc;
use crate::format::atoms::format_wing;
use crate::format::hoon::{format_hoon, format_hoon_wide};
use crate::format::mode::FormatMode;
use crate::format::symbols::*;
use crate::format::{rune2, rune_vararg, tall_form, wide_form, Format, RuneStyle};
use parser::ast::hoon::*;

/// Format a Spec to a Doc (tall mode - default).
pub fn format_spec(spec: &Spec, config: &FormatterConfig) -> Doc {
    format_spec_impl(spec, config, FormatMode::Tall)
}

/// Format a Spec in wide mode (for use inside () and []).
pub fn format_spec_wide(spec: &Spec, config: &FormatterConfig) -> Doc {
    format_spec_impl(spec, config, FormatMode::Wide)
}

/// Internal implementation with explicit mode parameter.
pub fn format_spec_impl(spec: &Spec, config: &FormatterConfig, mode: FormatMode) -> Doc {
    let fmt = |s: &Spec| format_spec_impl(s, config, mode);
    let fmt_wide = |s: &Spec| format_spec_impl(s, config, FormatMode::Wide);
    let fmt_hoon = |h: &Hoon| {
        match mode {
            FormatMode::Tall => format_hoon(h, config),
            FormatMode::Wide => format_hoon_wide(h, config),
        }
    };
    let fmt_hoon_wide = |h: &Hoon| format_hoon_wide(h, config);
    let indent = config.indent_width as i32;

    match spec {
        // ===== Basic Forms (mode-agnostic) =====

        Spec::Base(base) => base.format(config),

        Spec::Dbug(_spot, inner) => {
            // Skip debug wrapper, just format inner
            fmt(inner)
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
                fmt(inner),
            ])
        }

        Spec::Make(hoon, args) => {
            // Constructed type - contents are inside () so always use wide mode
            let hoon_doc = format_hoon_wide(hoon, config);
            if args.is_empty() {
                Doc::concat(vec![Doc::text("("), hoon_doc, Doc::text(")")])
            } else {
                let arg_docs: Vec<Doc> = args.iter().map(|a| fmt_wide(a)).collect();
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
                fmt(inner),
            ])
        }

        Spec::Over(wing, inner) => {
            // Type derived from wing
            Doc::concat(vec![
                format_wing(wing, config),
                Doc::text("/"),
                fmt(inner),
            ])
        }

        // ===== Rune Specs =====

        // $> bucgar - filter by head
        Spec::BucGar(head, tail) => {
            match mode {
                FormatMode::Tall => tall_form(BUC_GAR, vec![fmt(head), fmt(tail)]),
                FormatMode::Wide => wide_form(BUC_GAR, vec![fmt_wide(head), fmt_wide(tail)]),
            }
        }

        // $$ bucbuc - recursive structure
        Spec::BucBuc(inner, arms) => {
            match mode {
                FormatMode::Tall => {
                    let inner_doc = fmt(inner);
                    let arm_docs: Vec<Doc> = arms
                        .iter()
                        .map(|(name, s)| {
                            Doc::concat(vec![
                                Doc::text(format!("++  {}", name)),
                                Doc::gap(),
                                fmt(s),
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
                FormatMode::Wide => {
                    let inner_doc = fmt_wide(inner);
                    let arm_docs: Vec<Doc> = arms
                        .iter()
                        .map(|(name, s)| {
                            Doc::concat(vec![
                                Doc::text(format!("++  {}", name)),
                                Doc::text(" "),
                                fmt_wide(s),
                            ])
                        })
                        .collect();
                    let mut children = vec![inner_doc];
                    children.extend(arm_docs);
                    wide_form(BUC_BUC, children)
                }
            }
        }

        // $| bucbar - normalizing gate
        Spec::BucBar(spec_inner, gate) => {
            match mode {
                FormatMode::Tall => tall_form(BUC_BAR, vec![fmt(spec_inner), fmt_hoon(gate)]),
                FormatMode::Wide => wide_form(BUC_BAR, vec![fmt_wide(spec_inner), fmt_hoon_wide(gate)]),
            }
        }

        // $_ buccab - example type
        Spec::BucCab(example) => {
            match mode {
                FormatMode::Tall => Doc::concat(vec![Doc::text(BUC_CAB), Doc::gap(), format_hoon(example, config)]),
                FormatMode::Wide => Doc::concat(vec![Doc::text("_"), fmt_hoon_wide(example)]),
            }
        }

        // $: buccol - tuple type
        Spec::BucCol(head, tail) => {
            match mode {
                FormatMode::Tall => {
                    // Tall mode with line breaks
                    let mut child_parts = vec![];
                    child_parts.push(Doc::hardline());
                    child_parts.push(fmt(head));
                    for s in tail {
                        child_parts.push(Doc::hardline());
                        child_parts.push(fmt(s));
                    }
                    Doc::nest(
                        indent,
                        Doc::concat(vec![
                            Doc::text(BUC_COL),
                            Doc::nest(indent, Doc::concat(child_parts)),
                            Doc::hardline(),
                            Doc::text("=="),
                        ]),
                    )
                }
                FormatMode::Wide => {
                    let mut children = vec![fmt_wide(head)];
                    children.extend(tail.iter().map(|s| fmt_wide(s)));
                    wide_form(BUC_COL, children)
                }
            }
        }

        // $% buccen - tagged union
        Spec::BucCen(head, cases) => {
            match mode {
                FormatMode::Tall => {
                    let mut children = vec![fmt(head)];
                    children.extend(cases.iter().map(|s| fmt(s)));
                    rune_vararg(BUC_CEN, children, indent)
                }
                FormatMode::Wide => {
                    let mut children = vec![fmt_wide(head)];
                    children.extend(cases.iter().map(|s| fmt_wide(s)));
                    wide_form(BUC_CEN, children)
                }
            }
        }

        // $. bucdot - cell with sample
        Spec::BucDot(spec_inner, arms) => {
            match mode {
                FormatMode::Tall => {
                    let spec_doc = fmt(spec_inner);
                    let arm_docs: Vec<Doc> = arms
                        .iter()
                        .map(|(name, s)| {
                            Doc::concat(vec![
                                Doc::text(format!("{}=", name)),
                                fmt(s),
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
                FormatMode::Wide => {
                    let spec_doc = fmt_wide(spec_inner);
                    let arm_docs: Vec<Doc> = arms
                        .iter()
                        .map(|(name, s)| {
                            Doc::concat(vec![
                                Doc::text(format!("{}=", name)),
                                fmt_wide(s),
                            ])
                        })
                        .collect();
                    let mut children = vec![spec_doc];
                    children.extend(arm_docs);
                    wide_form(BUC_DOT, children)
                }
            }
        }

        // $< bucgal - filter by tail
        Spec::BucGal(head, tail) => {
            match mode {
                FormatMode::Tall => tall_form(BUC_GAL, vec![fmt(head), fmt(tail)]),
                FormatMode::Wide => wide_form(BUC_GAL, vec![fmt_wide(head), fmt_wide(tail)]),
            }
        }

        // $- buchep - gate type
        Spec::BucHep(arg, ret) => {
            match mode {
                FormatMode::Tall => tall_form(BUC_HEP, vec![fmt(arg), fmt(ret)]),
                FormatMode::Wide => wide_form(BUC_HEP, vec![fmt_wide(arg), fmt_wide(ret)]),
            }
        }

        // $^ bucket - cell of types
        Spec::BucKet(head, tail) => {
            match mode {
                FormatMode::Tall => tall_form(BUC_KET, vec![fmt(head), fmt(tail)]),
                FormatMode::Wide => wide_form(BUC_KET, vec![fmt_wide(head), fmt_wide(tail)]),
            }
        }

        // $+ buclus - alias
        Spec::BucLus(name, inner) => {
            match mode {
                FormatMode::Tall => rune2(
                    BUC_LUS,
                    Doc::text(name.clone()),
                    fmt(inner),
                    indent,
                    RuneStyle::backstep_after_first(),
                ),
                FormatMode::Wide => wide_form(BUC_LUS, vec![Doc::text(name.clone()), fmt_wide(inner)]),
            }
        }

        // $/ bucfas - mold builder
        Spec::BucFas(spec_inner, arms) => {
            match mode {
                FormatMode::Tall => {
                    let spec_doc = fmt(spec_inner);
                    let arm_docs: Vec<Doc> = arms
                        .iter()
                        .map(|(name, s)| {
                            Doc::concat(vec![
                                Doc::text(format!("{}=", name)),
                                fmt(s),
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
                FormatMode::Wide => {
                    let spec_doc = fmt_wide(spec_inner);
                    let arm_docs: Vec<Doc> = arms
                        .iter()
                        .map(|(name, s)| {
                            Doc::concat(vec![
                                Doc::text(format!("{}=", name)),
                                fmt_wide(s),
                            ])
                        })
                        .collect();
                    let mut children = vec![spec_doc];
                    children.extend(arm_docs);
                    wide_form(BUC_FAS, children)
                }
            }
        }

        // $; bucmic - manual normalizer
        Spec::BucMic(gate) => {
            match mode {
                FormatMode::Tall => Doc::concat(vec![Doc::text(BUC_MIC), Doc::gap(), format_hoon(gate, config)]),
                FormatMode::Wide => wide_form(BUC_MIC, vec![fmt_hoon_wide(gate)]),
            }
        }

        // $& bucpam - pair test
        Spec::BucPam(spec_inner, test) => {
            match mode {
                FormatMode::Tall => tall_form(BUC_PAM, vec![fmt(spec_inner), fmt_hoon(test)]),
                FormatMode::Wide => wide_form(BUC_PAM, vec![fmt_wide(spec_inner), fmt_hoon_wide(test)]),
            }
        }

        // $~ bucsig - default value
        Spec::BucSig(default, inner) => {
            match mode {
                FormatMode::Tall => tall_form(BUC_SIG, vec![fmt_hoon(default), fmt(inner)]),
                FormatMode::Wide => wide_form(BUC_SIG, vec![fmt_hoon_wide(default), fmt_wide(inner)]),
            }
        }

        // $` buctis - face with wrap
        Spec::BucTic(spec_inner, arms) => {
            match mode {
                FormatMode::Tall => {
                    let spec_doc = fmt(spec_inner);
                    let arm_docs: Vec<Doc> = arms
                        .iter()
                        .map(|(name, s)| {
                            Doc::concat(vec![
                                Doc::text(format!("{}=", name)),
                                fmt(s),
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
                FormatMode::Wide => {
                    let spec_doc = fmt_wide(spec_inner);
                    let arm_docs: Vec<Doc> = arms
                        .iter()
                        .map(|(name, s)| {
                            Doc::concat(vec![
                                Doc::text(format!("{}=", name)),
                                fmt_wide(s),
                            ])
                        })
                        .collect();
                    let mut children = vec![spec_doc];
                    children.extend(arm_docs);
                    wide_form(BUC_TIC, children)
                }
            }
        }

        // $= buctis - face binding
        Spec::BucTis(skin, inner) => {
            match mode {
                FormatMode::Tall => {
                    Doc::concat(vec![
                        Doc::text(BUC_TIS),
                        Doc::gap(),
                        format_skin(skin, config),
                        Doc::gap(),
                        fmt(inner),
                    ])
                }
                FormatMode::Wide => wide_form(BUC_TIS, vec![format_skin(skin, config), fmt_wide(inner)]),
            }
        }

        // $@ bucpat - atom/cell test
        Spec::BucPat(atom_case, cell_case) => {
            match mode {
                FormatMode::Tall => tall_form(BUC_PAT, vec![fmt(atom_case), fmt(cell_case)]),
                FormatMode::Wide => wide_form(BUC_PAT, vec![fmt_wide(atom_case), fmt_wide(cell_case)]),
            }
        }

        // $? bucwut - union type
        Spec::BucWut(head, cases) => {
            match mode {
                FormatMode::Tall => {
                    let mut children = vec![fmt(head)];
                    children.extend(cases.iter().map(|s| fmt(s)));
                    rune_vararg(BUC_WUT, children, indent)
                }
                FormatMode::Wide => {
                    let mut children = vec![fmt_wide(head)];
                    children.extend(cases.iter().map(|s| fmt_wide(s)));
                    wide_form(BUC_WUT, children)
                }
            }
        }

        // $! buczap - mold test
        Spec::BucZap(spec_inner, arms) => {
            match mode {
                FormatMode::Tall => {
                    let spec_doc = fmt(spec_inner);
                    let arm_docs: Vec<Doc> = arms
                        .iter()
                        .map(|(name, s)| {
                            Doc::concat(vec![
                                Doc::text(format!("{}=", name)),
                                fmt(s),
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
                FormatMode::Wide => {
                    let spec_doc = fmt_wide(spec_inner);
                    let arm_docs: Vec<Doc> = arms
                        .iter()
                        .map(|(name, s)| {
                            Doc::concat(vec![
                                Doc::text(format!("{}=", name)),
                                fmt_wide(s),
                            ])
                        })
                        .collect();
                    let mut children = vec![spec_doc];
                    children.extend(arm_docs);
                    wide_form(BUC_ZAP, children)
                }
            }
        }

        // Preserved irregular syntax: [a=@ b=tape]
        // Contents are inside [] so always use wide mode for children
        Spec::CellLiteral(specs) => {
            let children: Vec<Doc> = specs.iter().map(|s| fmt_wide(s)).collect();
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
                fmt(inner),
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
