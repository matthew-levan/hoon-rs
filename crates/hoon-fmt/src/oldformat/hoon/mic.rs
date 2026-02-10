use parser::ast::hoon::Hoon;

use super::helpers::format_marl;
use super::FormatCtx;
use crate::doc::Doc;
use crate::format::mode::FormatMode;
use crate::format::symbols::*;
use crate::format::{rune4, rune_vararg, tall_form, wide_form, RuneStyle};

pub(super) fn format(hoon: &Hoon, ctx: FormatCtx<'_>) -> Option<Doc> {
    let fmt = |h: &Hoon| ctx.fmt(h);
    let fmt_wide = |h: &Hoon| ctx.fmt_wide(h);
    let fmt_spec = |s| ctx.fmt_spec(s);
    let fmt_spec_wide = |s| ctx.fmt_spec_wide(s);
    let indent = ctx.indent();

    match hoon {
        Hoon::MicTis(marl) => Some(format_marl(marl, ctx.config)),
        Hoon::MicCol(gate, hoons) => Some(match ctx.mode {
            FormatMode::Tall => {
                let mut children = vec![fmt(gate)];
                children.extend(hoons.iter().map(|h| fmt(h)));
                rune_vararg(MIC_COL, children, indent)
            }
            FormatMode::Wide => {
                let mut children = vec![fmt_wide(gate)];
                children.extend(hoons.iter().map(|h| fmt_wide(h)));
                wide_form(MIC_COL, children)
            }
        }),
        Hoon::MicFas(hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => Doc::concat(vec![Doc::text(MIC_FAS), Doc::gap(), fmt(hoon_inner)]),
            FormatMode::Wide => Doc::concat(vec![
                Doc::text(MIC_FAS),
                Doc::text("("),
                fmt_wide(hoon_inner),
                Doc::text(")"),
            ]),
        }),
        Hoon::MicGal(spec, a, b, c) => Some(match ctx.mode {
            FormatMode::Tall => {
                rune4(
                    MIC_GAL,
                    fmt_spec(spec),
                    fmt(a),
                    fmt(b),
                    fmt(c),
                    indent,
                    RuneStyle::backstep_before_first(),
                )
            }
            FormatMode::Wide => wide_form(
                MIC_GAL,
                vec![fmt_spec_wide(spec), fmt_wide(a), fmt_wide(b), fmt_wide(c)],
            ),
        }),
        Hoon::MicSig(gate, hoons) => Some(match ctx.mode {
            FormatMode::Tall => {
                let mut children = vec![fmt(gate)];
                children.extend(hoons.iter().map(|h| fmt(h)));
                rune_vararg(MIC_SIG, children, indent)
            }
            FormatMode::Wide => {
                let mut children = vec![fmt_wide(gate)];
                children.extend(hoons.iter().map(|h| fmt_wide(h)));
                wide_form(MIC_SIG, children)
            }
        }),
        Hoon::MicMic(spec, hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => tall_form(MIC_MIC, vec![fmt_spec(spec), fmt(hoon_inner)]),
            FormatMode::Wide => wide_form(MIC_MIC, vec![fmt_spec_wide(spec), fmt_wide(hoon_inner)]),
        }),
        _ => None,
    }
}
