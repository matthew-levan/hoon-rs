use parser::ast::hoon::Hoon;

use super::FormatCtx;
use crate::doc::Doc;
use crate::format::spec::format_skin;

pub(super) fn format(hoon: &Hoon, ctx: FormatCtx<'_>) -> Option<Doc> {
    let fmt = |h: &Hoon| ctx.fmt(h);
    let fmt_wide = |h: &Hoon| ctx.fmt_wide(h);

    match hoon {
        Hoon::Pair(left, right) => Some(Doc::concat(vec![
            Doc::text("["),
            fmt_wide(left),
            Doc::text(" "),
            fmt_wide(right),
            Doc::text("]"),
        ])),
        Hoon::CellLiteral(hoons) => {
            let children: Vec<Doc> = hoons.iter().map(|h| fmt_wide(h)).collect();
            Some(Doc::concat(vec![
                Doc::text("["),
                Doc::join(Doc::text(" "), children),
                Doc::text("]"),
            ]))
        }
        Hoon::ListLiteral(hoons) => {
            let children: Vec<Doc> = hoons.iter().map(|h| fmt_wide(h)).collect();
            Some(Doc::concat(vec![
                Doc::text("~["),
                Doc::join(Doc::text(" "), children),
                Doc::text("]"),
            ]))
        }
        Hoon::NameBinding(skin, hoon_inner) => Some(Doc::concat(vec![
            format_skin(skin, ctx.config),
            Doc::text("="),
            fmt(hoon_inner),
        ])),
        Hoon::FunctionCall(gate, args) => {
            let mut children = vec![fmt_wide(gate)];
            children.extend(args.iter().map(|h| fmt_wide(h)));
            Some(Doc::concat(vec![
                Doc::text("("),
                Doc::join(Doc::text(" "), children),
                Doc::text(")"),
            ]))
        }
        Hoon::ColonAccess(p, q) => Some(Doc::concat(vec![fmt(p), Doc::text(":"), fmt(q)])),
        _ => None,
    }
}
