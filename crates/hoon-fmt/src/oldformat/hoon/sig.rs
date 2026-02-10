use parser::ast::hoon::Hoon;

use super::helpers::format_tyre;
use super::FormatCtx;
use crate::doc::Doc;
use crate::format::mode::FormatMode;
use crate::format::symbols::*;
use crate::format::{rune2, rune3, rune4, tall_form, wide_form, Format, RuneStyle};

pub(super) fn format(hoon: &Hoon, ctx: FormatCtx<'_>) -> Option<Doc> {
    let fmt = |h: &Hoon| ctx.fmt(h);
    let fmt_wide = |h: &Hoon| ctx.fmt_wide(h);
    let indent = ctx.indent();

    match hoon {
        Hoon::SigBar(hint, hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => tall_form(SIG_BAR, vec![fmt(hint), fmt(hoon_inner)]),
            FormatMode::Wide => wide_form(SIG_BAR, vec![fmt_wide(hint), fmt_wide(hoon_inner)]),
        }),
        Hoon::SigCab(hint, hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => tall_form(SIG_CAB, vec![fmt(hint), fmt(hoon_inner)]),
            FormatMode::Wide => wide_form(SIG_CAB, vec![fmt_wide(hint), fmt_wide(hoon_inner)]),
        }),
        Hoon::SigCen(chum, base, tyre, hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => {
                let chum_doc = chum.format(ctx.config);
                let base_doc = fmt(base);
                let tyre_doc = format_tyre(tyre, ctx.config);
                let hoon_doc = fmt(hoon_inner);
                Doc::concat(vec![
                    Doc::text(SIG_CEN),
                    Doc::gap(),
                    chum_doc,
                    Doc::gap(),
                    base_doc,
                    Doc::gap(),
                    tyre_doc,
                    Doc::gap(),
                    hoon_doc,
                ])
            }
            FormatMode::Wide => {
                let chum_doc = chum.format(ctx.config);
                let base_doc = fmt_wide(base);
                let tyre_doc = format_tyre(tyre, ctx.config);
                let hoon_doc = fmt_wide(hoon_inner);
                wide_form(SIG_CEN, vec![chum_doc, base_doc, tyre_doc, hoon_doc])
            }
        }),
        Hoon::SigFas(chum, hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => tall_form(SIG_FAS, vec![chum.format(ctx.config), fmt(hoon_inner)]),
            FormatMode::Wide => {
                wide_form(SIG_FAS, vec![chum.format(ctx.config), fmt_wide(hoon_inner)])
            }
        }),
        Hoon::SigGal(term_or_pair, hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => tall_form(
                SIG_GAL,
                vec![term_or_pair.format(ctx.config), fmt(hoon_inner)],
            ),
            FormatMode::Wide => wide_form(
                SIG_GAL,
                vec![term_or_pair.format(ctx.config), fmt_wide(hoon_inner)],
            ),
        }),
        Hoon::SigGar(term_or_pair, hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => tall_form(
                SIG_GAR,
                vec![term_or_pair.format(ctx.config), fmt(hoon_inner)],
            ),
            FormatMode::Wide => wide_form(
                SIG_GAR,
                vec![term_or_pair.format(ctx.config), fmt_wide(hoon_inner)],
            ),
        }),
        Hoon::SigBuc(name, hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => rune2(
                SIG_BUC,
                Doc::text(format!("%{}", name)),
                fmt(hoon_inner),
                indent,
                RuneStyle::backstep_after_first(),
            ),
            FormatMode::Wide => wide_form(
                SIG_BUC,
                vec![Doc::text(format!("%{}", name)), fmt_wide(hoon_inner)],
            ),
        }),
        Hoon::SigLus(priority, hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => rune2(
                SIG_LUS,
                Doc::text(format!("{}", priority)),
                fmt(hoon_inner),
                indent,
                RuneStyle::backstep_after_first(),
            ),
            FormatMode::Wide => wide_form(
                SIG_LUS,
                vec![Doc::text(format!("{}", priority)), fmt_wide(hoon_inner)],
            ),
        }),
        Hoon::SigPam(priority, test, hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => rune3(
                SIG_PAM,
                Doc::text(format!("{}", priority)),
                fmt(test),
                fmt(hoon_inner),
                indent,
                RuneStyle::backstep_before_first(),
            ),
            FormatMode::Wide => wide_form(
                SIG_PAM,
                vec![Doc::text(format!("{}", priority)), fmt_wide(test), fmt_wide(hoon_inner)],
            ),
        }),
        Hoon::SigTis(a, b) => Some(match ctx.mode {
            FormatMode::Tall => tall_form(SIG_TIS, vec![fmt(a), fmt(b)]),
            FormatMode::Wide => wide_form(SIG_TIS, vec![fmt_wide(a), fmt_wide(b)]),
        }),
        Hoon::SigWut(priority, test, then, hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => rune4(
                SIG_WUT,
                Doc::text(format!("{}", priority)),
                fmt(test),
                fmt(then),
                fmt(hoon_inner),
                indent,
                RuneStyle::backstep_before_first(),
            ),
            FormatMode::Wide => wide_form(
                SIG_WUT,
                vec![
                    Doc::text(format!("{}", priority)),
                    fmt_wide(test),
                    fmt_wide(then),
                    fmt_wide(hoon_inner),
                ],
            ),
        }),
        Hoon::SigZap(hint, hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => tall_form(SIG_ZAP, vec![fmt(hint), fmt(hoon_inner)]),
            FormatMode::Wide => wide_form(SIG_ZAP, vec![fmt_wide(hint), fmt_wide(hoon_inner)]),
        }),
        _ => None,
    }
}
