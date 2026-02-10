use parser::ast::hoon::Hoon;

use super::FormatCtx;
use crate::doc::Doc;
use crate::format::atoms::format_wing;
use crate::format::mode::FormatMode;
use crate::format::symbols::*;
use crate::format::spec::format_skin;
use crate::format::{rune3, rune_vararg, tall_form, wide_form, RuneStyle};

pub(super) fn format(hoon: &Hoon, ctx: FormatCtx<'_>) -> Option<Doc> {
    let fmt = |h: &Hoon| ctx.fmt(h);
    let fmt_wide = |h: &Hoon| ctx.fmt_wide(h);
    let fmt_spec = |s| ctx.fmt_spec(s);
    let fmt_spec_wide = |s| ctx.fmt_spec_wide(s);
    let indent = ctx.indent();

    match hoon {
        Hoon::WutBar(hoons) => Some(match ctx.mode {
            FormatMode::Tall => {
                let children: Vec<Doc> = hoons.iter().map(|h| fmt(h)).collect();
                rune_vararg(WUT_BAR, children, indent)
            }
            FormatMode::Wide => {
                let children: Vec<Doc> = hoons.iter().map(|h| fmt_wide(h)).collect();
                wide_form(WUT_BAR, children)
            }
        }),
        Hoon::WutHep(wing, cases) => Some(match ctx.mode {
            FormatMode::Tall => {
                let wing_doc = format_wing(wing, ctx.config);
                let case_docs: Vec<Doc> = cases
                    .iter()
                    .map(|(spec, hoon_case)| {
                        Doc::concat(vec![ctx.fmt_spec(spec), Doc::text("  "), fmt(hoon_case)])
                    })
                    .collect();
                Doc::concat(vec![
                    Doc::text(WUT_HEP),
                    Doc::gap(),
                    wing_doc,
                    Doc::nest(
                        indent,
                        Doc::concat(
                            case_docs
                                .into_iter()
                                .map(|d| Doc::concat(vec![Doc::hardline(), d]))
                                .collect(),
                        ),
                    ),
                    Doc::hardline(),
                    Doc::text("=="),
                ])
            }
            FormatMode::Wide => {
                let wing_doc = format_wing(wing, ctx.config);
                let case_docs: Vec<Doc> = cases
                    .iter()
                    .map(|(spec, hoon_case)| {
                        Doc::concat(vec![
                            fmt_spec_wide(spec),
                            Doc::text(" "),
                            fmt_wide(hoon_case),
                        ])
                    })
                    .collect();
                let mut children = vec![wing_doc];
                children.extend(case_docs);
                wide_form(WUT_HEP, children)
            }
        }),
        Hoon::WutCol(test, yes, no) => Some(match ctx.mode {
            FormatMode::Tall => rune3(
                WUT_COL,
                fmt(test),
                fmt(yes),
                fmt(no),
                indent,
                RuneStyle::backstep_before_first(),
            ),
            FormatMode::Wide => {
                wide_form(WUT_COL, vec![fmt_wide(test), fmt_wide(yes), fmt_wide(no)])
            }
        }),
        Hoon::WutDot(test, yes, no) => Some(match ctx.mode {
            FormatMode::Tall => rune3(
                WUT_DOT,
                fmt(test),
                fmt(yes),
                fmt(no),
                indent,
                RuneStyle::backstep_before_first(),
            ),
            FormatMode::Wide => {
                wide_form(WUT_DOT, vec![fmt_wide(test), fmt_wide(yes), fmt_wide(no)])
            }
        }),
        Hoon::WutKet(wing, yes, no) => Some(match ctx.mode {
            FormatMode::Tall => rune3(
                WUT_KET,
                format_wing(wing, ctx.config),
                fmt(yes),
                fmt(no),
                indent,
                RuneStyle::backstep_before_first(),
            ),
            FormatMode::Wide => wide_form(
                WUT_KET,
                vec![format_wing(wing, ctx.config), fmt_wide(yes), fmt_wide(no)],
            ),
        }),
        Hoon::WutGal(test, crash) => Some(match ctx.mode {
            FormatMode::Tall => tall_form(WUT_GAL, vec![fmt(test), fmt(crash)]),
            FormatMode::Wide => wide_form(WUT_GAL, vec![fmt_wide(test), fmt_wide(crash)]),
        }),
        Hoon::WutGar(test, pass) => Some(match ctx.mode {
            FormatMode::Tall => tall_form(WUT_GAR, vec![fmt(test), fmt(pass)]),
            FormatMode::Wide => wide_form(WUT_GAR, vec![fmt_wide(test), fmt_wide(pass)]),
        }),
        Hoon::WutLus(wing, default, cases) => Some(match ctx.mode {
            FormatMode::Tall => {
                let wing_doc = format_wing(wing, ctx.config);
                let default_doc = fmt(default);
                let case_docs: Vec<Doc> = cases
                    .iter()
                    .map(|(spec, hoon_case)| {
                        Doc::concat(vec![ctx.fmt_spec(spec), Doc::text("  "), fmt(hoon_case)])
                    })
                    .collect();
                Doc::concat(vec![
                    Doc::text(WUT_LUS),
                    Doc::gap(),
                    wing_doc,
                    Doc::gap(),
                    default_doc,
                    Doc::nest(
                        indent,
                        Doc::concat(
                            case_docs
                                .into_iter()
                                .map(|d| Doc::concat(vec![Doc::hardline(), d]))
                                .collect(),
                        ),
                    ),
                    Doc::hardline(),
                    Doc::text("=="),
                ])
            }
            FormatMode::Wide => {
                let wing_doc = format_wing(wing, ctx.config);
                let default_doc = fmt_wide(default);
                let case_docs: Vec<Doc> = cases
                    .iter()
                    .map(|(spec, hoon_case)| {
                        Doc::concat(vec![
                            fmt_spec_wide(spec),
                            Doc::text(" "),
                            fmt_wide(hoon_case),
                        ])
                    })
                    .collect();
                let mut children = vec![wing_doc, default_doc];
                children.extend(case_docs);
                wide_form(WUT_LUS, children)
            }
        }),
        Hoon::WutPam(hoons) => Some(match ctx.mode {
            FormatMode::Tall => {
                let children: Vec<Doc> = hoons.iter().map(|h| fmt(h)).collect();
                rune_vararg(WUT_PAM, children, indent)
            }
            FormatMode::Wide => {
                let children: Vec<Doc> = hoons.iter().map(|h| fmt_wide(h)).collect();
                wide_form(WUT_PAM, children)
            }
        }),
        Hoon::WutPat(wing, yes, no) => Some(match ctx.mode {
            FormatMode::Tall => rune3(
                WUT_PAT,
                format_wing(wing, ctx.config),
                fmt(yes),
                fmt(no),
                indent,
                RuneStyle::backstep_before_first(),
            ),
            FormatMode::Wide => wide_form(
                WUT_PAT,
                vec![format_wing(wing, ctx.config), fmt_wide(yes), fmt_wide(no)],
            ),
        }),
        Hoon::WutSig(wing, yes, no) => Some(match ctx.mode {
            FormatMode::Tall => rune3(
                WUT_SIG,
                format_wing(wing, ctx.config),
                fmt(yes),
                fmt(no),
                indent,
                RuneStyle::backstep_before_first(),
            ),
            FormatMode::Wide => wide_form(
                WUT_SIG,
                vec![format_wing(wing, ctx.config), fmt_wide(yes), fmt_wide(no)],
            ),
        }),
        Hoon::WutHax(skin, wing) => Some(match ctx.mode {
            FormatMode::Tall => tall_form(
                WUT_HAX,
                vec![format_skin(skin, ctx.config), format_wing(wing, ctx.config)],
            ),
            FormatMode::Wide => wide_form(
                WUT_HAX,
                vec![format_skin(skin, ctx.config), format_wing(wing, ctx.config)],
            ),
        }),
        Hoon::WutTis(spec, wing) => Some(match ctx.mode {
            FormatMode::Tall => {
                tall_form(WUT_TIS, vec![fmt_spec(spec), format_wing(wing, ctx.config)])
            }
            FormatMode::Wide => wide_form(
                WUT_TIS,
                vec![fmt_spec_wide(spec), format_wing(wing, ctx.config)],
            ),
        }),
        Hoon::WutZap(hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => Doc::concat(vec![Doc::text(WUT_ZAP), Doc::gap(), fmt(hoon_inner)]),
            FormatMode::Wide => Doc::concat(vec![Doc::text("!"), fmt_wide(hoon_inner)]),
        }),
        _ => None,
    }
}
