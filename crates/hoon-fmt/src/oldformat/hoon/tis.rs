use parser::ast::hoon::Hoon;

use super::FormatCtx;
use crate::doc::Doc;
use crate::format::atoms::format_wing;
use crate::format::mode::FormatMode;
use crate::format::spec::format_skin;
use crate::format::symbols::*;
use crate::format::{rune2, rune3, rune4, wide_form, RuneStyle};

pub(super) fn format(hoon: &Hoon, ctx: FormatCtx<'_>) -> Option<Doc> {
    let fmt = |h: &Hoon| ctx.fmt(h);
    let fmt_wide = |h: &Hoon| ctx.fmt_wide(h);
    let fmt_spec = |s| ctx.fmt_spec(s);
    let fmt_spec_wide = |s| ctx.fmt_spec_wide(s);
    let indent = ctx.indent();

    match hoon {
        Hoon::TisBar(spec, hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => rune2(
                TIS_BAR,
                fmt_spec(spec),
                fmt(hoon_inner),
                indent,
                RuneStyle::leading_after_first(),
            ),
            FormatMode::Wide => wide_form(TIS_BAR, vec![fmt_spec_wide(spec), fmt_wide(hoon_inner)]),
        }),
        Hoon::TisCol(updates, hoon_inner) => Some(match ctx.mode {
            FormatMode::Tall => {
                let update_docs: Vec<Doc> = updates
                    .iter()
                    .map(|(w, h)| {
                        Doc::concat(vec![format_wing(w, ctx.config), Doc::text("  "), fmt(h)])
                    })
                    .collect();
                Doc::concat(vec![
                    Doc::hardline(),
                    Doc::text(TIS_COL),
                    Doc::hardline(),
                    Doc::nest(
                        indent,
                        Doc::join(Doc::concat(vec![Doc::hardline()]), update_docs),
                    ),
                    Doc::hardline(),
                    Doc::text("=="),
                    Doc::hardline(),
                    fmt(hoon_inner),
                ])
            }
            FormatMode::Wide => {
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
                let mut children = update_docs;
                children.push(fmt_wide(hoon_inner));
                wide_form(TIS_COL, children)
            }
        }),
        Hoon::TisFas(skin, value, body) => {
            let header_flat = Doc::concat(vec![
                Doc::text(TIS_FAS),
                Doc::Gap,
                format_skin(skin, ctx.config),
                Doc::Gap,
                fmt(value),
            ]);
            let header_broken = Doc::concat(vec![
                Doc::text(TIS_FAS),
                Doc::line(),
                Doc::nest(
                    indent,
                    Doc::concat(vec![
                        format_skin(skin, ctx.config),
                        Doc::text("  "),
                        fmt(value),
                    ]),
                ),
            ]);
            let header = Doc::group(Doc::flat_alt(header_broken, header_flat));
            Some(match ctx.mode {
                FormatMode::Tall => {
                    Doc::concat(vec![Doc::hardline(), header, Doc::hardline(), fmt(body)])
                }
                FormatMode::Wide => wide_form(
                    TIS_FAS,
                    vec![format_skin(skin, ctx.config), fmt_wide(value), fmt_wide(body)],
                ),
            })
        }
        Hoon::TisMic(skin, value, body) => {
            let header_flat = Doc::concat(vec![
                Doc::text(TIS_MIC),
                Doc::text("  "),
                format_skin(skin, ctx.config),
                Doc::text("  "),
                fmt(value),
            ]);
            let header_broken = Doc::concat(vec![
                Doc::text(TIS_MIC),
                Doc::line(),
                Doc::nest(
                    indent,
                    Doc::concat(vec![
                        format_skin(skin, ctx.config),
                        Doc::text("  "),
                        fmt(value),
                    ]),
                ),
            ]);
            let header = Doc::group(Doc::flat_alt(header_broken, header_flat));
            Some(match ctx.mode {
                FormatMode::Tall => {
                    Doc::concat(vec![Doc::hardline(), header, Doc::hardline(), fmt(body)])
                }
                FormatMode::Wide => wide_form(
                    TIS_MIC,
                    vec![format_skin(skin, ctx.config), fmt_wide(value), fmt_wide(body)],
                ),
            })
        }
        Hoon::TisDot(wing, value, body) => Some(match ctx.mode {
            FormatMode::Tall => rune3(
                TIS_DOT,
                format_wing(wing, ctx.config),
                fmt(value),
                fmt(body),
                indent,
                RuneStyle::block_after_first(),
            ),
            FormatMode::Wide => wide_form(
                TIS_DOT,
                vec![format_wing(wing, ctx.config), fmt_wide(value), fmt_wide(body)],
            ),
        }),
        Hoon::TisWut(wing, test, yes, no) => Some(match ctx.mode {
            FormatMode::Tall => rune4(
                TIS_WUT,
                format_wing(wing, ctx.config),
                fmt(test),
                fmt(yes),
                fmt(no),
                indent,
                RuneStyle::leading_before_first(),
            ),
            FormatMode::Wide => wide_form(
                TIS_WUT,
                vec![format_wing(wing, ctx.config), fmt_wide(test), fmt_wide(yes), fmt_wide(no)],
            ),
        }),
        Hoon::TisGal(a, b) => Some(match ctx.mode {
            FormatMode::Tall => rune2(
                TIS_GAL,
                fmt(a),
                fmt(b),
                indent,
                RuneStyle::leading_after_first(),
            ),
            FormatMode::Wide => wide_form(TIS_GAL, vec![fmt_wide(a), fmt_wide(b)]),
        }),
        Hoon::TisHep(a, b) => Some(match ctx.mode {
            FormatMode::Tall => rune2(
                TIS_HEP,
                fmt(a),
                fmt(b),
                indent,
                RuneStyle::leading_after_first(),
            ),
            FormatMode::Wide => wide_form(TIS_HEP, vec![fmt_wide(a), fmt_wide(b)]),
        }),
        Hoon::TisGar(a, b) => Some(match ctx.mode {
            FormatMode::Tall => rune2(
                TIS_GAR,
                fmt(a),
                fmt(b),
                indent,
                RuneStyle::leading_after_first(),
            ),
            FormatMode::Wide => wide_form(TIS_GAR, vec![fmt_wide(a), fmt_wide(b)]),
        }),
        Hoon::TisKet(skin, wing, value, body) => Some(match ctx.mode {
            FormatMode::Tall => rune4(
                TIS_KET,
                format_skin(skin, ctx.config),
                format_wing(wing, ctx.config),
                fmt(value),
                fmt(body),
                indent,
                RuneStyle::leading_before_first(),
            ),
            FormatMode::Wide => wide_form(
                TIS_KET,
                vec![
                    format_skin(skin, ctx.config),
                    format_wing(wing, ctx.config),
                    fmt_wide(value),
                    fmt_wide(body),
                ],
            ),
        }),
        Hoon::TisLus(value, body) => Some(match ctx.mode {
            FormatMode::Tall => rune2(
                TIS_LUS,
                fmt(value),
                fmt(body),
                indent,
                RuneStyle::leading_after_first(),
            ),
            FormatMode::Wide => wide_form(TIS_LUS, vec![fmt_wide(value), fmt_wide(body)]),
        }),
        Hoon::TisSig(hoons) => {
            let children: Vec<Doc> = hoons
                .iter()
                .map(|h| super::format_hoon(h, ctx.config))
                .collect();
            Some(Doc::join(Doc::hardline(), children))
        }
        Hoon::TisTar((name, spec), value, body) => {
            let name_doc = if let Some(s) = spec {
                Doc::concat(vec![Doc::text(name.clone()), Doc::text("/"), fmt_spec(s)])
            } else {
                Doc::text(name.clone())
            };
            Some(match ctx.mode {
                FormatMode::Tall => rune3(
                    TIS_TAR,
                    name_doc,
                    fmt(value),
                    fmt(body),
                    indent,
                    RuneStyle::leading_after_second(),
                ),
                FormatMode::Wide => {
                    wide_form(TIS_TAR, vec![name_doc, fmt_wide(value), fmt_wide(body)])
                }
            })
        }
        Hoon::TisCom(a, b) => Some(match ctx.mode {
            FormatMode::Tall => rune2(
                TIS_COM,
                fmt(a),
                fmt(b),
                indent,
                RuneStyle::leading_after_first(),
            ),
            FormatMode::Wide => wide_form(TIS_COM, vec![fmt_wide(a), fmt_wide(b)]),
        }),
        _ => None,
    }
}
