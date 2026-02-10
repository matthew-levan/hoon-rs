use parser::ast::hoon::Hoon;

use super::FormatCtx;
use crate::doc::Doc;
use crate::format::mode::FormatMode;
use crate::format::symbols::*;
use crate::format::{rune2, tall_form, wide_form, RuneStyle};

pub(super) fn format(hoon: &Hoon, ctx: FormatCtx<'_>) -> Option<Doc> {
    let fmt = |h: &Hoon| ctx.fmt(h);
    let fmt_wide = |h: &Hoon| ctx.fmt_wide(h);
    let fmt_spec = |s| ctx.fmt_spec(s);
    let fmt_spec_wide = |s| ctx.fmt_spec_wide(s);
    let indent = ctx.indent();

    match hoon {
        Hoon::KetBar(hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => Doc::concat(vec![Doc::text(KET_BAR), Doc::gap(), fmt(hoon_inner)]),
            FormatMode::Wide => wide_form(KET_BAR, vec![fmt_wide(hoon_inner)]),
        }),
        Hoon::KetDot(a, b) => Some(match ctx.mode {
            FormatMode::Tall => tall_form(KET_DOT, vec![fmt(a), fmt(b)]),
            FormatMode::Wide => wide_form(KET_DOT, vec![fmt_wide(a), fmt_wide(b)]),
        }),
        Hoon::KetLus(a, b) => Some(match ctx.mode {
            FormatMode::Tall => tall_form(KET_LUS, vec![fmt(a), fmt(b)]),
            FormatMode::Wide => wide_form(KET_LUS, vec![fmt_wide(a), fmt_wide(b)]),
        }),
        Hoon::KetHep(spec, hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => rune2(
                KET_HEP,
                fmt_spec(spec),
                fmt(hoon_inner),
                indent,
                RuneStyle::backstep_after_first(),
            ),
            FormatMode::Wide => wide_form(KET_HEP, vec![fmt_spec_wide(spec), fmt_wide(hoon_inner)]),
        }),
        Hoon::KetPam(hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => Doc::concat(vec![Doc::text(KET_PAM), Doc::gap(), fmt(hoon_inner)]),
            FormatMode::Wide => wide_form(KET_PAM, vec![fmt_wide(hoon_inner)]),
        }),
        Hoon::KetSig(hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => Doc::concat(vec![Doc::text(KET_SIG), Doc::gap(), fmt(hoon_inner)]),
            FormatMode::Wide => wide_form(KET_SIG, vec![fmt_wide(hoon_inner)]),
        }),
        Hoon::KetTis(skin, hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => {
                let skin_doc = crate::format::spec::format_skin(skin, ctx.config);
                tall_form(KET_TIS, vec![skin_doc, fmt(hoon_inner)])
            }
            FormatMode::Wide => {
                let skin_doc = crate::format::spec::format_skin(skin, ctx.config);
                wide_form(KET_TIS, vec![skin_doc, fmt_wide(hoon_inner)])
            }
        }),
        Hoon::KetWut(hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => Doc::concat(vec![Doc::text(KET_WUT), Doc::gap(), fmt(hoon_inner)]),
            FormatMode::Wide => wide_form(KET_WUT, vec![fmt_wide(hoon_inner)]),
        }),
        Hoon::KetTar(spec) => Some(match ctx.mode {
            FormatMode::Tall => Doc::concat(vec![Doc::text(KET_TAR), Doc::gap(), fmt_spec(spec)]),
            FormatMode::Wide => Doc::concat(vec![Doc::text("*"), fmt_spec_wide(spec)]),
        }),
        Hoon::KetCol(spec) => Some(match ctx.mode {
            FormatMode::Tall => Doc::concat(vec![Doc::text(KET_COL), Doc::gap(), fmt_spec(spec)]),
            FormatMode::Wide => Doc::concat(vec![
                Doc::text(KET_COL),
                Doc::text("("),
                fmt_spec_wide(spec),
                Doc::text(")"),
            ]),
        }),
        _ => None,
    }
}
