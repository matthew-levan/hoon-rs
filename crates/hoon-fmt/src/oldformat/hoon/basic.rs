use parser::ast::hoon::{Hoon, TermOrTune, Woof};

use super::helpers::{
    format_axis, format_cord_char, format_leaf, format_manx, format_rock, format_sand,
};
use super::FormatCtx;
use crate::doc::Doc;
use crate::format::atoms::format_wing;
use crate::format::Format;

pub(super) fn format(hoon: &Hoon, ctx: FormatCtx<'_>) -> Option<Doc> {
    let fmt = |h: &Hoon| ctx.fmt(h);
    let fmt_wide = |h: &Hoon| ctx.fmt_wide(h);

    match hoon {
        Hoon::ZapZap => Some(Doc::text("!!")),
        Hoon::Axis(n) => Some(format_axis(*n)),
        Hoon::Base(base) => Some(base.format(ctx.config)),
        Hoon::Bust(base) => Some(Doc::concat(vec![Doc::text("*"), base.format(ctx.config)])),
        Hoon::Dbug(_spot, inner) => Some(fmt(inner)),
        Hoon::Eror(msg) => Some(Doc::text(format!("!!  {}", msg))),
        Hoon::Hand(_type, _nock) => Some(Doc::text("!!hand")),
        Hoon::Note(note, inner) => Some(Doc::concat(vec![
            note.format(ctx.config),
            Doc::text("/"),
            fmt(inner),
        ])),
        Hoon::Fits(hoon_inner, wing) => Some(Doc::concat(vec![
            Doc::text("?=("),
            fmt_wide(hoon_inner),
            Doc::text(" "),
            format_wing(wing, ctx.config),
            Doc::text(")"),
        ])),
        Hoon::Knit(woofs) => {
            let mut parts = vec![Doc::text("\"")];
            for woof in woofs {
                match woof {
                    Woof::ParsedAtom(a) => parts.push(format_cord_char(a)),
                    Woof::Hoon(h) => {
                        parts.push(Doc::text("{"));
                        parts.push(fmt_wide(h));
                        parts.push(Doc::text("}"));
                    }
                }
            }
            parts.push(Doc::text("\""));
            Some(Doc::concat(parts))
        }
        Hoon::Leaf(aura, value) => Some(format_leaf(aura, value, ctx.config)),
        Hoon::Limb(name) => Some(Doc::text(name.clone())),
        Hoon::Lost(inner) => Some(fmt(inner)),
        Hoon::Rock(aura, value) => Some(format_rock(aura, value, ctx.config)),
        Hoon::Sand(aura, value) => Some(format_sand(aura, value, ctx.config)),
        Hoon::Tell(hoons) => {
            let parts: Vec<Doc> = hoons.iter().map(|h| fmt_wide(h)).collect();
            Some(Doc::concat(vec![
                Doc::text("<"),
                Doc::join(Doc::text(" "), parts),
                Doc::text(">"),
            ]))
        }
        Hoon::Tune(term_or_tune) => Some(match term_or_tune {
            TermOrTune::Term(t) => Doc::text(format!("${}", t)),
            TermOrTune::Tune(_) => Doc::text("$tune"),
        }),
        Hoon::Wing(wing) => Some(format_wing(wing, ctx.config)),
        Hoon::Yell(hoons) => {
            let parts: Vec<Doc> = hoons.iter().map(|h| fmt_wide(h)).collect();
            Some(Doc::concat(vec![
                Doc::text(">"),
                Doc::join(Doc::text(" "), parts),
                Doc::text("<"),
            ]))
        }
        Hoon::Xray(manx) => Some(format_manx(manx, ctx.config)),
        _ => None,
    }
}
