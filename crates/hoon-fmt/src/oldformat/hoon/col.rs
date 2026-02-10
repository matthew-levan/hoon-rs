use parser::ast::hoon::Hoon;

use super::FormatCtx;
use crate::doc::Doc;
use crate::format::mode::FormatMode;
use crate::format::symbols::*;
use crate::format::{rune3, rune4, rune_vararg, tall_form, wide_form, RuneStyle};

pub(super) fn format(hoon: &Hoon, ctx: FormatCtx<'_>) -> Option<Doc> {
    let fmt = |h: &Hoon| ctx.fmt(h);
    let fmt_wide = |h: &Hoon| ctx.fmt_wide(h);
    let indent = ctx.indent();

    match hoon {
        Hoon::ColCab(head, tail) => Some(match ctx.mode {
            FormatMode::Tall => tall_form(COL_CAB, vec![fmt(head), fmt(tail)]),
            FormatMode::Wide => wide_form(COL_CAB, vec![fmt_wide(head), fmt_wide(tail)]),
        }),
        Hoon::ColKet(a, b, c, d) => Some(match ctx.mode {
            FormatMode::Tall => rune4(
                COL_KET,
                fmt(a),
                fmt(b),
                fmt(c),
                fmt(d),
                indent,
                RuneStyle::backstep_before_first(),
            ),
            FormatMode::Wide => wide_form(
                COL_KET,
                vec![fmt_wide(a), fmt_wide(b), fmt_wide(c), fmt_wide(d)],
            ),
        }),
        Hoon::ColHep(head, tail) => Some(match ctx.mode {
            FormatMode::Tall => tall_form(COL_HEP, vec![fmt(head), fmt(tail)]),
            FormatMode::Wide => wide_form(COL_HEP, vec![fmt_wide(head), fmt_wide(tail)]),
        }),
        Hoon::ColLus(a, b, c) => Some(match ctx.mode {
            FormatMode::Tall => rune3(
                COL_LUS,
                fmt(a),
                fmt(b),
                fmt(c),
                indent,
                RuneStyle::backstep_before_first(),
            ),
            FormatMode::Wide => wide_form(COL_LUS, vec![fmt_wide(a), fmt_wide(b), fmt_wide(c)]),
        }),
        Hoon::ColSig(hoons) => Some(match ctx.mode {
            FormatMode::Tall => {
                let children: Vec<Doc> = hoons.iter().map(|h| fmt(h)).collect();
                rune_vararg(COL_SIG, children, indent)
            }
            FormatMode::Wide => {
                let children: Vec<Doc> = hoons.iter().map(|h| fmt_wide(h)).collect();
                wide_form(COL_SIG, children)
            }
        }),
        Hoon::ColTar(hoons) => Some(match ctx.mode {
            FormatMode::Tall => {
                let children: Vec<Doc> = hoons.iter().map(|h| fmt(h)).collect();
                rune_vararg(COL_TAR, children, indent)
            }
            FormatMode::Wide => {
                let children: Vec<Doc> = hoons.iter().map(|h| fmt_wide(h)).collect();
                wide_form(COL_TAR, children)
            }
        }),
        _ => None,
    }
}
