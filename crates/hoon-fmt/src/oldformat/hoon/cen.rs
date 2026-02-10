use parser::ast::hoon::Hoon;

use super::FormatCtx;
use crate::doc::Doc;
use crate::format::atoms::format_wing;
use crate::format::mode::FormatMode;
use crate::format::symbols::*;
use crate::format::{rune2, rune3, rune4, rune_vararg, tall_form, wide_form, RuneStyle};

pub(super) fn format(hoon: &Hoon, ctx: FormatCtx<'_>) -> Option<Doc> {
    let fmt = |h: &Hoon| ctx.fmt(h);
    let fmt_wide = |h: &Hoon| ctx.fmt_wide(h);
    let indent = ctx.indent();

    match hoon {
        Hoon::CenCab(wing, updates) => Some(match ctx.mode {
            FormatMode::Tall => {
                let wing_doc = format_wing(wing, ctx.config);
                let update_docs: Vec<Doc> = updates
                    .iter()
                    .map(|(w, h)| {
                        Doc::concat(vec![format_wing(w, ctx.config), Doc::text("  "), fmt(h)])
                    })
                    .collect();
                Doc::concat(vec![
                    Doc::text(CEN_CAB),
                    Doc::gap(),
                    wing_doc,
                    Doc::gap(),
                    Doc::join(Doc::gap(), update_docs),
                    Doc::gap(),
                    Doc::text("=="),
                ])
            }
            FormatMode::Wide => {
                let wing_doc = format_wing(wing, ctx.config);
                let update_docs: Vec<Doc> = updates
                    .iter()
                    .map(|(w, h)| {
                        Doc::concat(vec![
                            format_wing(w, ctx.config),
                            Doc::text(" "),
                            fmt_wide(h),
                        ])
                    })
                    .collect();
                Doc::concat(vec![
                    Doc::text(CEN_CAB),
                    Doc::text("("),
                    wing_doc,
                    Doc::text(" "),
                    Doc::join(Doc::text(", "), update_docs),
                    Doc::text(")"),
                ])
            }
        }),
        Hoon::CenDot(gate, sample) => Some(match ctx.mode {
            FormatMode::Tall => tall_form(CEN_DOT, vec![fmt(gate), fmt(sample)]),
            FormatMode::Wide => wide_form(CEN_DOT, vec![fmt_wide(gate), fmt_wide(sample)]),
        }),
        Hoon::CenHep(gate, sample) => Some(match ctx.mode {
            FormatMode::Tall => rune2(
                CEN_HEP,
                fmt(gate),
                fmt(sample),
                indent,
                RuneStyle::backstep_after_first(),
            ),
            FormatMode::Wide => wide_form(CEN_HEP, vec![fmt_wide(gate), fmt_wide(sample)]),
        }),
        Hoon::CenCol(gate, samples) => Some(match ctx.mode {
            FormatMode::Tall => {
                let mut children = vec![fmt(gate)];
                children.extend(samples.iter().map(|h| fmt(h)));
                rune_vararg(CEN_COL, children, indent)
            }
            FormatMode::Wide => {
                let mut children = vec![fmt_wide(gate)];
                children.extend(samples.iter().map(|h| fmt_wide(h)));
                wide_form(CEN_COL, children)
            }
        }),
        Hoon::CenTar(wing, sample, updates) => Some(match ctx.mode {
            FormatMode::Tall => {
                let wing_doc = format_wing(wing, ctx.config);
                let sample_doc = fmt(sample);
                let update_docs: Vec<Doc> = updates
                    .iter()
                    .map(|(w, h)| {
                        Doc::concat(vec![format_wing(w, ctx.config), Doc::text("  "), fmt(h)])
                    })
                    .collect();
                Doc::concat(vec![
                    Doc::text(CEN_TAR),
                    Doc::gap(),
                    wing_doc,
                    Doc::gap(),
                    sample_doc,
                    Doc::gap(),
                    Doc::join(Doc::gap(), update_docs),
                    Doc::gap(),
                    Doc::text("=="),
                ])
            }
            FormatMode::Wide => {
                let wing_doc = format_wing(wing, ctx.config);
                let sample_doc = fmt_wide(sample);
                let update_docs: Vec<Doc> = updates
                    .iter()
                    .map(|(w, h)| {
                        Doc::concat(vec![
                            format_wing(w, ctx.config),
                            Doc::text(" "),
                            fmt_wide(h),
                        ])
                    })
                    .collect();
                let mut children = vec![wing_doc, sample_doc];
                children.extend(update_docs);
                wide_form(CEN_TAR, children)
            }
        }),
        Hoon::CenKet(gate, a, b, c) => Some(match ctx.mode {
            FormatMode::Tall => rune4(
                CEN_KET,
                fmt(gate),
                fmt(a),
                fmt(b),
                fmt(c),
                indent,
                RuneStyle::backstep_before_first(),
            ),
            FormatMode::Wide => wide_form(
                CEN_KET,
                vec![fmt_wide(gate), fmt_wide(a), fmt_wide(b), fmt_wide(c)],
            ),
        }),
        Hoon::CenLus(gate, a, b) => Some(match ctx.mode {
            FormatMode::Tall => rune3(
                CEN_LUS,
                fmt(gate),
                fmt(a),
                fmt(b),
                indent,
                RuneStyle::backstep_before_first(),
            ),
            FormatMode::Wide => wide_form(CEN_LUS, vec![fmt_wide(gate), fmt_wide(a), fmt_wide(b)]),
        }),
        Hoon::CenSig(wing, door, args) => Some(match ctx.mode {
            FormatMode::Tall => {
                let mut children = vec![format_wing(wing, ctx.config), fmt(door)];
                children.extend(args.iter().map(|h| fmt(h)));
                tall_form(CEN_SIG, children)
            }
            FormatMode::Wide => {
                let mut children = vec![format_wing(wing, ctx.config), fmt_wide(door)];
                children.extend(args.iter().map(|h| fmt_wide(h)));
                wide_form(CEN_SIG, children)
            }
        }),
        Hoon::CensigIrregular(wing, door, args) => {
            let mut children = vec![format_wing(wing, ctx.config), fmt_wide(door)];
            children.extend(args.iter().map(|h| fmt_wide(h)));
            Some(Doc::concat(vec![
                Doc::text("~("),
                Doc::join(Doc::text(" "), children),
                Doc::text(")"),
            ]))
        }
        Hoon::CenTis(wing, updates) => Some(match ctx.mode {
            FormatMode::Tall => {
                let wing_doc = format_wing(wing, ctx.config);
                let update_docs: Vec<Doc> = updates
                    .iter()
                    .map(|(w, h)| {
                        Doc::concat(vec![format_wing(w, ctx.config), Doc::text("  "), fmt(h)])
                    })
                    .collect();
                Doc::concat(vec![
                    Doc::text(CEN_TIS),
                    Doc::gap(),
                    wing_doc,
                    Doc::group(Doc::flat_alt(
                        Doc::concat(vec![
                            Doc::nest(
                                indent,
                                Doc::concat(
                                    update_docs
                                        .iter()
                                        .cloned()
                                        .map(|d| Doc::concat(vec![Doc::hardline(), d]))
                                        .collect(),
                                ),
                            ),
                            Doc::hardline(),
                            Doc::text("=="),
                        ]),
                        Doc::concat(vec![
                            Doc::text("  "),
                            Doc::join(Doc::text("  "), update_docs),
                            Doc::text("  "),
                            Doc::text("=="),
                        ]),
                    )),
                ])
            }
            FormatMode::Wide => {
                let wing_doc = format_wing(wing, ctx.config);
                let update_docs: Vec<Doc> = updates
                    .iter()
                    .map(|(w, h)| {
                        Doc::concat(vec![
                            format_wing(w, ctx.config),
                            Doc::text(" "),
                            fmt_wide(h),
                        ])
                    })
                    .collect();
                let mut children = vec![wing_doc];
                children.extend(update_docs);
                wide_form(CEN_TIS, children)
            }
        }),
        Hoon::CentisIrregular(wing, updates) => {
            if updates.is_empty() {
                Some(format_wing(wing, ctx.config))
            } else {
                let update_docs: Vec<Doc> = updates
                    .iter()
                    .map(|(w, h)| {
                        Doc::concat(vec![
                            format_wing(w, ctx.config),
                            Doc::text(" "),
                            fmt_wide(h),
                        ])
                    })
                    .collect();
                Some(Doc::concat(vec![
                    format_wing(wing, ctx.config),
                    Doc::text("("),
                    Doc::join(Doc::text(", "), update_docs),
                    Doc::text(")"),
                ]))
            }
        }
        _ => None,
    }
}
