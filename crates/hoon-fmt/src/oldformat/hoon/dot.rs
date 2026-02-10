use parser::ast::hoon::Hoon;

use super::FormatCtx;
use crate::doc::Doc;
use crate::format::mode::FormatMode;
use crate::format::symbols::*;
use crate::format::{tall_form, wide_form};

pub(super) fn format(hoon: &Hoon, ctx: FormatCtx<'_>) -> Option<Doc> {
    let fmt = |h: &Hoon| ctx.fmt(h);
    let fmt_wide = |h: &Hoon| ctx.fmt_wide(h);
    let fmt_spec = |s| ctx.fmt_spec(s);
    let fmt_spec_wide = |s| ctx.fmt_spec_wide(s);

    match hoon {
        Hoon::DotKet(spec, hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => tall_form(DOT_KET, vec![fmt_spec(spec), fmt(hoon_inner)]),
            FormatMode::Wide => wide_form(DOT_KET, vec![fmt_spec_wide(spec), fmt_wide(hoon_inner)]),
        }),
        Hoon::DotLus(hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => Doc::concat(vec![Doc::text(DOT_LUS), Doc::gap(), fmt(hoon_inner)]),
            FormatMode::Wide => wide_form(DOT_LUS, vec![fmt_wide(hoon_inner)]),
        }),
        Hoon::DotTar(formula, subject) => Some(match ctx.mode {
            FormatMode::Tall => tall_form(DOT_TAR, vec![fmt(formula), fmt(subject)]),
            FormatMode::Wide => wide_form(DOT_TAR, vec![fmt_wide(formula), fmt_wide(subject)]),
        }),
        Hoon::DotTis(a, b) => Some(match ctx.mode {
            FormatMode::Tall => tall_form(DOT_TIS, vec![fmt(a), fmt(b)]),
            FormatMode::Wide => wide_form(DOT_TIS, vec![fmt_wide(a), fmt_wide(b)]),
        }),
        Hoon::DotWut(hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => Doc::concat(vec![Doc::text(DOT_WUT), Doc::gap(), fmt(hoon_inner)]),
            FormatMode::Wide => Doc::concat(vec![
                Doc::text(".?"),
                Doc::text("("),
                fmt_wide(hoon_inner),
                Doc::text(")"),
            ]),
        }),
        _ => None,
    }
}
