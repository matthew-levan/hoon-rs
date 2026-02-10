use parser::ast::hoon::Hoon;

use super::helpers::{format_arms, format_core, format_door};
use super::FormatCtx;
use crate::doc::Doc;
use crate::format::mode::FormatMode;
use crate::format::symbols::*;
use crate::format::{rune2, wide_form, RuneStyle};

pub(super) fn format(hoon: &Hoon, ctx: FormatCtx<'_>) -> Option<Doc> {
    let fmt = |h: &Hoon| ctx.fmt(h);
    let fmt_wide = |h: &Hoon| ctx.fmt_wide(h);
    let fmt_spec = |s| ctx.fmt_spec(s);
    let fmt_spec_wide = |s| ctx.fmt_spec_wide(s);
    let indent = ctx.indent();

    match hoon {
        Hoon::BarBuc(names, spec) => {
            let names_doc = Doc::join(
                Doc::text(" "),
                names.iter().map(|n| Doc::text(n.clone())).collect(),
            );
            Some(match ctx.mode {
                FormatMode::Tall => rune2(
                    BAR_BUC,
                    names_doc,
                    fmt_spec(spec),
                    indent,
                    RuneStyle::leading_after_first(),
                ),
                FormatMode::Wide => wide_form(BAR_BUC, vec![names_doc, fmt_spec_wide(spec)]),
            })
        }
        Hoon::BarCab(spec, alas, arms) => Some(format_door(BAR_CAB, spec, alas, arms, ctx.config)),
        Hoon::BarCol(sample, body) => Some(match ctx.mode {
            FormatMode::Tall => rune2(
                BAR_COL,
                fmt(sample),
                fmt(body),
                indent,
                RuneStyle::leading_after_first(),
            ),
            FormatMode::Wide => wide_form(BAR_COL, vec![fmt_wide(sample), fmt_wide(body)]),
        }),
        Hoon::BarCen(name, arms) => Some(format_core(BAR_CEN, name, arms, ctx.config)),
        Hoon::BarDot(body) => Some(match ctx.mode {
            FormatMode::Tall => Doc::concat(vec![
                Doc::hardline(),
                Doc::text(BAR_DOT),
                Doc::hardline(),
                fmt(body),
            ]),
            FormatMode::Wide => wide_form(BAR_DOT, vec![fmt_wide(body)]),
        }),
        Hoon::BarKet(body, arms) => {
            let body_doc = super::format_hoon(body, ctx.config);
            let arms_doc = format_arms(arms, ctx.config);
            Some(Doc::concat(vec![
                Doc::hardline(),
                Doc::text(BAR_KET),
                Doc::nest(indent, Doc::concat(vec![Doc::hardline(), body_doc, arms_doc])),
                Doc::hardline(),
                Doc::text("--"),
                Doc::hardline(),
            ]))
        }
        Hoon::BarHep(body) => Some(match ctx.mode {
            FormatMode::Tall => Doc::concat(vec![
                Doc::hardline(),
                Doc::text(BAR_HEP),
                Doc::hardline(),
                fmt(body),
            ]),
            FormatMode::Wide => wide_form(BAR_HEP, vec![fmt_wide(body)]),
        }),
        Hoon::BarSig(spec, body) => Some(match ctx.mode {
            FormatMode::Tall => rune2(
                BAR_SIG,
                fmt_spec(spec),
                fmt(body),
                indent,
                RuneStyle::leading_after_first(),
            ),
            FormatMode::Wide => wide_form(BAR_SIG, vec![fmt_spec_wide(spec), fmt_wide(body)]),
        }),
        Hoon::BarTar(spec, body) => Some(match ctx.mode {
            FormatMode::Tall => rune2(
                BAR_TAR,
                fmt_spec(spec),
                fmt(body),
                indent,
                RuneStyle::leading_after_first(),
            ),
            FormatMode::Wide => wide_form(BAR_TAR, vec![fmt_spec_wide(spec), fmt_wide(body)]),
        }),
        Hoon::BarTis(spec, body) => Some(match ctx.mode {
            // FormatMode::Tall => Doc::concat(vec![
            //     Doc::hardline(),
            //     Doc::text(BAR_TIS),
            //     Doc::hardline(),
            //     fmt_spec(spec),
            //     Doc::nest(indent, Doc::concat(vec![Doc::hardline(), fmt(body)])),
            // ]),
            FormatMode::Tall => rune2(
                BAR_TIS,
                fmt_spec(spec),
                fmt(body),
                indent,
                RuneStyle::block_after_first(),
            ),
            FormatMode::Wide => wide_form(BAR_TIS, vec![fmt_spec_wide(spec), fmt_wide(body)]),
        }),
        Hoon::BarPat(name, arms) => Some(format_core(BAR_PAT, name, arms, ctx.config)),
        Hoon::BarWut(body) => Some(match ctx.mode {
            FormatMode::Tall => Doc::concat(vec![
                Doc::hardline(),
                Doc::text(BAR_WUT),
                Doc::hardline(),
                fmt(body),
            ]),
            FormatMode::Wide => wide_form(BAR_WUT, vec![fmt_wide(body)]),
        }),
        _ => None,
    }
}
