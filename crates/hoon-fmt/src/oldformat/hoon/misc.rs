use parser::ast::hoon::Hoon;

use super::FormatCtx;
use crate::doc::Doc;
use crate::format::mode::FormatMode;

pub(super) fn format(hoon: &Hoon, ctx: FormatCtx<'_>) -> Option<Doc> {
    match hoon {
        Hoon::LusBuc(spec) => Some(ctx.fmt_spec(spec)),
        Hoon::WithImports(imports, body) => Some(Doc::concat(vec![
            Doc::text(imports.trim_end()),
            Doc::hardline(),
            super::format_hoon(body, ctx.config),
        ])),
        Hoon::Wide(inner) => Some(match ctx.mode {
            FormatMode::Wide => ctx.fmt(inner),
            FormatMode::Tall => super::format_hoon_wide(inner, ctx.config),
        }),
        _ => None,
    }
}
