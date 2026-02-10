use parser::ast::hoon::{Hoon, ZpwtArg};

use super::FormatCtx;
use crate::doc::Doc;
use crate::format::atoms::format_wing;
use crate::format::mode::FormatMode;
use crate::format::symbols::*;
use crate::format::{rune2, rune3, tall_form, wide_form, RuneStyle};

pub(super) fn format(hoon: &Hoon, ctx: FormatCtx<'_>) -> Option<Doc> {
    let fmt = |h: &Hoon| ctx.fmt(h);
    let fmt_wide = |h: &Hoon| ctx.fmt_wide(h);
    let fmt_spec = |s| ctx.fmt_spec(s);
    let fmt_spec_wide = |s| ctx.fmt_spec_wide(s);
    let indent = ctx.indent();

    match hoon {
        Hoon::ZapCom(a, b) => Some(match ctx.mode {
            FormatMode::Tall => tall_form(ZAP_COM, vec![fmt(a), fmt(b)]),
            FormatMode::Wide => wide_form(ZAP_COM, vec![fmt_wide(a), fmt_wide(b)]),
        }),
        Hoon::ZapGar(hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => Doc::concat(vec![Doc::text("!>"), Doc::gap(), fmt(hoon_inner)]),
            FormatMode::Wide => wide_form("!>", vec![fmt_wide(hoon_inner)]),
        }),
        Hoon::ZapGal(spec, hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => tall_form(ZAP_GAL, vec![fmt_spec(spec), fmt(hoon_inner)]),
            FormatMode::Wide => wide_form(ZAP_GAL, vec![fmt_spec_wide(spec), fmt_wide(hoon_inner)]),
        }),
        Hoon::ZapMic(a, b) => Some(match ctx.mode {
            FormatMode::Tall => tall_form(ZAP_MIC, vec![fmt(a), fmt(b)]),
            FormatMode::Wide => wide_form(ZAP_MIC, vec![fmt_wide(a), fmt_wide(b)]),
        }),
        Hoon::ZapTis(hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => Doc::concat(vec![Doc::text(ZAP_TIS), Doc::gap(), fmt(hoon_inner)]),
            FormatMode::Wide => Doc::concat(vec![
                Doc::text("!="),
                Doc::text("("),
                fmt_wide(hoon_inner),
                Doc::text(")"),
            ]),
        }),
        Hoon::ZapPat(wings, yes, no) => {
            let wings_doc = Doc::join(
                Doc::text(" "),
                wings.iter().map(|w| format_wing(w, ctx.config)).collect(),
            );
            Some(match ctx.mode {
                FormatMode::Tall => rune3(
                    ZAP_PAT,
                    wings_doc,
                    fmt(yes),
                    fmt(no),
                    indent,
                    RuneStyle::backstep_before_first(),
                ),
                FormatMode::Wide => {
                    wide_form(ZAP_PAT, vec![wings_doc, fmt_wide(yes), fmt_wide(no)])
                }
            })
        }
        Hoon::ZapWut(arg, hoon_inner) => {
            let arg_doc = match arg {
                ZpwtArg::ParsedAtom(s) => Doc::text(format!("%{}", s)),
                ZpwtArg::Pair(a, b) => Doc::text(format!("[%{} %{}]", a, b)),
            };
            Some(match ctx.mode {
                FormatMode::Tall => rune2(
                    ZAP_WUT,
                    arg_doc,
                    fmt(hoon_inner),
                    indent,
                    RuneStyle::backstep_after_first(),
                ),
                FormatMode::Wide => fmt_wide(hoon_inner),
            })
        }
        _ => None,
    }
}
