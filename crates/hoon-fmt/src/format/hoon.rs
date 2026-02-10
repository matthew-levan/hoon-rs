//! Formatting for Hoon AST nodes.
//!
//! This module handles all ~118 variants of the Hoon enum.

use crate::config::FormatterConfig;
use crate::doc::Doc;
use crate::format::atoms::{format_noun_expr, format_wing};
use crate::format::runes::*;
use crate::format::spec::{format_skin, format_spec};
use crate::format::{
    rune2_backstep, rune3_backstep, rune4_backstep, rune_vararg, tall_or_wide, Format,
};
use indexmap::IndexMap;
use parser::ast::hoon::*;

/// Format a Hoon AST node to a Doc.
pub fn format_hoon(hoon: &Hoon, config: &FormatterConfig) -> Doc {
    match hoon {
        // ===== Basic Forms =====

        Hoon::Pair(left, right) => {
            // Cell constructor [a b]
            Doc::concat(vec![
                Doc::text("["),
                format_hoon(left, config),
                Doc::text(" "),
                format_hoon(right, config),
                Doc::text("]"),
            ])
        }

        Hoon::ZapZap => Doc::text("!!"),

        Hoon::Axis(n) => format_axis(*n),

        Hoon::Base(base) => base.format(config),

        Hoon::Bust(base) => {
            // *base - bunt of a type
            Doc::concat(vec![Doc::text("*"), base.format(config)])
        }

        Hoon::Dbug(_spot, inner) => {
            // Debug wrapper - just format inner
            format_hoon(inner, config)
        }

        Hoon::Eror(msg) => {
            // Error placeholder
            Doc::text(format!("!!  {}", msg))
        }

        Hoon::Hand(_type, _nock) => {
            // Hand-compiled nock - can't really format this
            Doc::text("!!hand")
        }

        Hoon::Note(note, inner) => {
            // Note hint wrapper
            Doc::concat(vec![
                note.format(config),
                Doc::text("/"),
                format_hoon(inner, config),
            ])
        }

        Hoon::Fits(hoon, wing) => {
            // ?=(spec wing) type test
            Doc::concat(vec![
                Doc::text("?=("),
                format_hoon(hoon, config),
                Doc::text(" "),
                format_wing(wing, config),
                Doc::text(")"),
            ])
        }

        Hoon::Knit(woofs) => {
            // Interpolated string "foo{bar}baz"
            let mut parts = vec![Doc::text("\"")];
            for woof in woofs {
                match woof {
                    Woof::ParsedAtom(a) => {
                        parts.push(format_cord_char(a));
                    }
                    Woof::Hoon(h) => {
                        parts.push(Doc::text("{"));
                        parts.push(format_hoon(h, config));
                        parts.push(Doc::text("}"));
                    }
                }
            }
            parts.push(Doc::text("\""));
            Doc::concat(parts)
        }

        Hoon::Leaf(aura, value) => {
            // Constant value with aura
            format_leaf(aura, value, config)
        }

        Hoon::Limb(name) => Doc::text(name.clone()),

        Hoon::Lost(inner) => {
            // Lost computation
            format_hoon(inner, config)
        }

        Hoon::Rock(aura, value) => {
            // Constant
            format_rock(aura, value, config)
        }

        Hoon::Sand(aura, value) => {
            // Constant (like rock but different category)
            format_sand(aura, value, config)
        }

        Hoon::Tell(hoons) => {
            // <a b c> render as tape
            let parts: Vec<Doc> = hoons.iter().map(|h| format_hoon(h, config)).collect();
            Doc::concat(vec![
                Doc::text("<"),
                Doc::join(Doc::text(" "), parts),
                Doc::text(">"),
            ])
        }

        Hoon::Tune(term_or_tune) => {
            match term_or_tune {
                TermOrTune::Term(t) => Doc::text(format!("${}", t)),
                TermOrTune::Tune(_) => Doc::text("$tune"),
            }
        }

        Hoon::Wing(wing) => format_wing(wing, config),

        Hoon::Yell(hoons) => {
            // >a b c< render as tank
            let parts: Vec<Doc> = hoons.iter().map(|h| format_hoon(h, config)).collect();
            Doc::concat(vec![
                Doc::text(">"),
                Doc::join(Doc::text(" "), parts),
                Doc::text("<"),
            ])
        }

        Hoon::Xray(manx) => format_manx(manx, config),

        // ===== Bar | Runes - Cores =====

        Hoon::BarBuc(names, spec) => {
            // |$ - wet gate mold
            let names_doc = Doc::join(Doc::text(" "), names.iter().map(|n| Doc::text(n.clone())).collect());
            rune2_backstep(
                BAR_BUC,
                names_doc,
                format_spec(spec, config),
                config.indent_width as i32,
            )
        }

        Hoon::BarCab(spec, alas, arms) => {
            // |_ - door
            format_door(BAR_CAB, spec, alas, arms, config)
        }

        Hoon::BarCol(sample, body) => {
            // |: - gate with custom sample
            tall_or_wide(
                BAR_COL,
                vec![format_hoon(sample, config), format_hoon(body, config)],
            )
        }

        Hoon::BarCen(name, arms) => {
            // |% - core
            format_core(BAR_CEN, name, arms, config)
        }

        Hoon::BarDot(body) => {
            // |. - trap
            Doc::concat(vec![Doc::text(BAR_DOT), Doc::gap(), format_hoon(body, config)])
        }

        Hoon::BarKet(body, arms) => {
            // |^ - core with main arm
            let body_doc = format_hoon(body, config);
            let arms_doc = format_arms(arms, config);
            Doc::concat(vec![
                Doc::text(BAR_KET),
                Doc::gap(),
                body_doc,
                arms_doc,
                Doc::gap(),
                Doc::text("--"),
            ])
        }

        Hoon::BarHep(body) => {
            // |- - loop/trap with kick
            Doc::concat(vec![Doc::text(BAR_HEP), Doc::gap(), format_hoon(body, config)])
        }

        Hoon::BarSig(spec, body) => {
            // |~ - iron gate
            rune2_backstep(
                BAR_SIG,
                format_spec(spec, config),
                format_hoon(body, config),
                config.indent_width as i32,
            )
        }

        Hoon::BarTar(spec, body) => {
            // |* - wet gate
            rune2_backstep(
                BAR_TAR,
                format_spec(spec, config),
                format_hoon(body, config),
                config.indent_width as i32,
            )
        }

        Hoon::BarTis(spec, body) => {
            // |= - gate
            rune2_backstep(
                BAR_TIS,
                format_spec(spec, config),
                format_hoon(body, config),
                config.indent_width as i32,
            )
        }

        Hoon::BarPat(name, arms) => {
            // |@ - jet hint core
            format_core(BAR_PAT, name, arms, config)
        }

        Hoon::BarWut(body) => {
            // |? - lead trap
            Doc::concat(vec![Doc::text(BAR_WUT), Doc::gap(), format_hoon(body, config)])
        }

        // ===== Col : Runes - Cells =====

        Hoon::ColCab(head, tail) => {
            // :_ - reversed cell
            tall_or_wide(
                COL_CAB,
                vec![format_hoon(head, config), format_hoon(tail, config)],
            )
        }

        Hoon::ColKet(a, b, c, d) => {
            // :^ - 4-tuple
            rune4_backstep(
                COL_KET,
                format_hoon(a, config),
                format_hoon(b, config),
                format_hoon(c, config),
                format_hoon(d, config),
                config.indent_width as i32,
            )
        }

        Hoon::ColHep(head, tail) => {
            // :- - cell
            tall_or_wide(
                COL_HEP,
                vec![format_hoon(head, config), format_hoon(tail, config)],
            )
        }

        Hoon::ColLus(a, b, c) => {
            // :+ - triple
            rune3_backstep(
                COL_LUS,
                format_hoon(a, config),
                format_hoon(b, config),
                format_hoon(c, config),
                config.indent_width as i32,
            )
        }

        Hoon::ColSig(hoons) => {
            // :~ - null-terminated list
            let children: Vec<Doc> = hoons.iter().map(|h| format_hoon(h, config)).collect();
            rune_vararg(COL_SIG, children, config.indent_width as i32)
        }

        Hoon::ColTar(hoons) => {
            // :* - tuple (regular form)
            let children: Vec<Doc> = hoons.iter().map(|h| format_hoon(h, config)).collect();
            rune_vararg(COL_TAR, children, config.indent_width as i32)
        }

        // ===== Cen % Runes - Calls =====

        Hoon::CenCab(wing, updates) => {
            // %_ - resolve with changes
            let wing_doc = format_wing(wing, config);
            let update_docs: Vec<Doc> = updates
                .iter()
                .map(|(w, h)| {
                    Doc::concat(vec![
                        format_wing(w, config),
                        Doc::text("  "),
                        format_hoon(h, config),
                    ])
                })
                .collect();
            Doc::concat(vec![
                Doc::text(CEN_CAB),
                Doc::gap(),
                wing_doc,
                Doc::gap(),
                Doc::join(Doc::gap(), update_docs),
                Doc::gap(),
                Doc::text("=="),
            ])
        }

        Hoon::CenDot(gate, sample) => {
            // %. - reverse call
            tall_or_wide(
                CEN_DOT,
                vec![format_hoon(gate, config), format_hoon(sample, config)],
            )
        }

        Hoon::CenHep(gate, sample) => {
            // %- - call gate
            tall_or_wide(
                CEN_HEP,
                vec![format_hoon(gate, config), format_hoon(sample, config)],
            )
        }

        Hoon::CenCol(gate, samples) => {
            // %: - call with n args
            let mut children = vec![format_hoon(gate, config)];
            children.extend(samples.iter().map(|h| format_hoon(h, config)));
            rune_vararg(CEN_COL, children, config.indent_width as i32)
        }

        Hoon::CenTar(wing, sample, updates) => {
            // %* - wing call with changes
            let wing_doc = format_wing(wing, config);
            let sample_doc = format_hoon(sample, config);
            let update_docs: Vec<Doc> = updates
                .iter()
                .map(|(w, h)| {
                    Doc::concat(vec![
                        format_wing(w, config),
                        Doc::text("  "),
                        format_hoon(h, config),
                    ])
                })
                .collect();
            Doc::concat(vec![
                Doc::text(CEN_TAR),
                Doc::gap(),
                wing_doc,
                Doc::gap(),
                sample_doc,
                Doc::gap(),
                Doc::join(Doc::gap(), update_docs),
                Doc::gap(),
                Doc::text("=="),
            ])
        }

        Hoon::CenKet(gate, a, b, c) => {
            // %^ - call with 3-tuple sample
            rune4_backstep(
                CEN_KET,
                format_hoon(gate, config),
                format_hoon(a, config),
                format_hoon(b, config),
                format_hoon(c, config),
                config.indent_width as i32,
            )
        }

        Hoon::CenLus(gate, a, b) => {
            // %+ - call with 2-tuple sample
            rune3_backstep(
                CEN_LUS,
                format_hoon(gate, config),
                format_hoon(a, config),
                format_hoon(b, config),
                config.indent_width as i32,
            )
        }

        Hoon::CenSig(wing, door, args) => {
            // %~ - call door arm
            let mut children = vec![format_wing(wing, config), format_hoon(door, config)];
            children.extend(args.iter().map(|h| format_hoon(h, config)));
            rune_vararg(CEN_SIG, children, config.indent_width as i32)
        }

        Hoon::CenTis(wing, updates) => {
            // %= - resolve with changes
            let wing_doc = format_wing(wing, config);
            let update_docs: Vec<Doc> = updates
                .iter()
                .map(|(w, h)| {
                    Doc::concat(vec![
                        format_wing(w, config),
                        Doc::text("  "),
                        format_hoon(h, config),
                    ])
                })
                .collect();
            Doc::concat(vec![
                Doc::text(CEN_TIS),
                Doc::gap(),
                wing_doc,
                Doc::gap(),
                Doc::join(Doc::gap(), update_docs),
                Doc::gap(),
                Doc::text("=="),
            ])
        }

        // ===== Dot . Runes - Nock =====

        Hoon::DotKet(spec, hoon) => {
            // .^ - scry
            tall_or_wide(
                DOT_KET,
                vec![format_spec(spec, config), format_hoon(hoon, config)],
            )
        }

        Hoon::DotLus(hoon) => {
            // .+ - increment
            Doc::concat(vec![Doc::text(DOT_LUS), Doc::gap(), format_hoon(hoon, config)])
        }

        Hoon::DotTar(formula, subject) => {
            // .* - nock
            tall_or_wide(
                DOT_TAR,
                vec![format_hoon(formula, config), format_hoon(subject, config)],
            )
        }

        Hoon::DotTis(a, b) => {
            // .= - equality test
            tall_or_wide(
                DOT_TIS,
                vec![format_hoon(a, config), format_hoon(b, config)],
            )
        }

        Hoon::DotWut(hoon) => {
            // .? - cell test
            Doc::concat(vec![Doc::text(DOT_WUT), Doc::gap(), format_hoon(hoon, config)])
        }

        // ===== Ket ^ Runes - Casts =====

        Hoon::KetBar(hoon) => {
            // ^| - make iron
            Doc::concat(vec![Doc::text(KET_BAR), Doc::gap(), format_hoon(hoon, config)])
        }

        Hoon::KetDot(a, b) => {
            // ^. - cell constructor? (rare)
            tall_or_wide(
                KET_DOT,
                vec![format_hoon(a, config), format_hoon(b, config)],
            )
        }

        Hoon::KetLus(a, b) => {
            // ^+ - type from example
            tall_or_wide(
                KET_LUS,
                vec![format_hoon(a, config), format_hoon(b, config)],
            )
        }

        Hoon::KetHep(spec, hoon) => {
            // ^- - cast to type (prefer flat/gap mode)
            Doc::concat(vec![
                Doc::text(KET_HEP),
                Doc::gap(),
                format_spec(spec, config),
                Doc::gap(),
                format_hoon(hoon, config),
            ])
        }

        Hoon::KetPam(hoon) => {
            // ^& - make zinc
            Doc::concat(vec![Doc::text(KET_PAM), Doc::gap(), format_hoon(hoon, config)])
        }

        Hoon::KetSig(hoon) => {
            // ^~ - constant fold
            Doc::concat(vec![Doc::text(KET_SIG), Doc::gap(), format_hoon(hoon, config)])
        }

        Hoon::KetTis(skin, hoon) => {
            // ^= - name binding (regular form)
            tall_or_wide(
                KET_TIS,
                vec![format_skin(skin, config), format_hoon(hoon, config)],
            )
        }

        Hoon::KetWut(hoon) => {
            // ^? - make lead
            Doc::concat(vec![Doc::text(KET_WUT), Doc::gap(), format_hoon(hoon, config)])
        }

        Hoon::KetTar(spec) => {
            // ^* - bunt
            Doc::concat(vec![Doc::text(KET_TAR), Doc::gap(), format_spec(spec, config)])
        }

        Hoon::KetCol(spec) => {
            // ^: - mold from spec
            Doc::concat(vec![Doc::text(KET_COL), Doc::gap(), format_spec(spec, config)])
        }

        // ===== Sig ~ Runes - Hints =====

        Hoon::SigBar(hint, hoon) => {
            // ~| - tracing printf
            tall_or_wide(
                SIG_BAR,
                vec![format_hoon(hint, config), format_hoon(hoon, config)],
            )
        }

        Hoon::SigCab(hint, hoon) => {
            // ~_ - slog hint
            tall_or_wide(
                SIG_CAB,
                vec![format_hoon(hint, config), format_hoon(hoon, config)],
            )
        }

        Hoon::SigCen(chum, base, tyre, hoon) => {
            // ~% - jet registration
            let chum_doc = chum.format(config);
            let base_doc = format_hoon(base, config);
            let tyre_doc = format_tyre(tyre, config);
            let hoon_doc = format_hoon(hoon, config);
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

        Hoon::SigFas(chum, hoon) => {
            // ~/ - jet hint
            tall_or_wide(
                SIG_FAS,
                vec![chum.format(config), format_hoon(hoon, config)],
            )
        }

        Hoon::SigGal(term_or_pair, hoon) => {
            // ~< - raw hint
            tall_or_wide(
                SIG_GAL,
                vec![term_or_pair.format(config), format_hoon(hoon, config)],
            )
        }

        Hoon::SigGar(term_or_pair, hoon) => {
            // ~> - raw hint
            tall_or_wide(
                SIG_GAR,
                vec![term_or_pair.format(config), format_hoon(hoon, config)],
            )
        }

        Hoon::SigBuc(name, hoon) => {
            // ~$ - profiling hint
            rune2_backstep(
                SIG_BUC,
                Doc::text(format!("%{}", name)),
                format_hoon(hoon, config),
                config.indent_width as i32,
            )
        }

        Hoon::SigLus(priority, hoon) => {
            // ~+ - cache hint
            rune2_backstep(
                SIG_LUS,
                Doc::text(format!("{}", priority)),
                format_hoon(hoon, config),
                config.indent_width as i32,
            )
        }

        Hoon::SigPam(priority, test, hoon) => {
            // ~& - conditional printf
            rune3_backstep(
                SIG_PAM,
                Doc::text(format!("{}", priority)),
                format_hoon(test, config),
                format_hoon(hoon, config),
                config.indent_width as i32,
            )
        }

        Hoon::SigTis(a, b) => {
            // ~= - dedupe hint
            tall_or_wide(
                SIG_TIS,
                vec![format_hoon(a, config), format_hoon(b, config)],
            )
        }

        Hoon::SigWut(priority, test, then, hoon) => {
            // ~? - conditional printf
            rune4_backstep(
                SIG_WUT,
                Doc::text(format!("{}", priority)),
                format_hoon(test, config),
                format_hoon(then, config),
                format_hoon(hoon, config),
                config.indent_width as i32,
            )
        }

        Hoon::SigZap(hint, hoon) => {
            // ~! - type printing
            tall_or_wide(
                SIG_ZAP,
                vec![format_hoon(hint, config), format_hoon(hoon, config)],
            )
        }

        // ===== Mic ; Runes - Make =====

        Hoon::MicTis(marl) => {
            // ;= - sail template
            format_marl(marl, config)
        }

        Hoon::MicCol(gate, hoons) => {
            // ;: - slam loop
            let mut children = vec![format_hoon(gate, config)];
            children.extend(hoons.iter().map(|h| format_hoon(h, config)));
            rune_vararg(MIC_COL, children, config.indent_width as i32)
        }

        Hoon::MicFas(hoon) => {
            // ;/ - tape interpolation
            Doc::concat(vec![Doc::text(MIC_FAS), Doc::gap(), format_hoon(hoon, config)])
        }

        Hoon::MicGal(spec, a, b, c) => {
            // ;< - monadic bind
            rune4_backstep(
                MIC_GAL,
                format_spec(spec, config),
                format_hoon(a, config),
                format_hoon(b, config),
                format_hoon(c, config),
                config.indent_width as i32,
            )
        }

        Hoon::MicSig(gate, hoons) => {
            // ;~ - kleisli compose
            let mut children = vec![format_hoon(gate, config)];
            children.extend(hoons.iter().map(|h| format_hoon(h, config)));
            rune_vararg(MIC_SIG, children, config.indent_width as i32)
        }

        Hoon::MicMic(spec, hoon) => {
            // ;; - normalize
            tall_or_wide(
                MIC_MIC,
                vec![format_spec(spec, config), format_hoon(hoon, config)],
            )
        }

        // ===== Tis = Runes - Compositions =====

        Hoon::TisBar(spec, hoon) => {
            // =| - pin default
            rune2_backstep(
                TIS_BAR,
                format_spec(spec, config),
                format_hoon(hoon, config),
                config.indent_width as i32,
            )
        }

        Hoon::TisCol(updates, hoon) => {
            // =: - batch update
            let update_docs: Vec<Doc> = updates
                .iter()
                .map(|(w, h)| {
                    Doc::concat(vec![
                        format_wing(w, config),
                        Doc::text("  "),
                        format_hoon(h, config),
                    ])
                })
                .collect();
            Doc::concat(vec![
                Doc::text(TIS_COL),
                Doc::gap(),
                Doc::join(Doc::gap(), update_docs),
                Doc::gap(),
                Doc::text("=="),
                Doc::gap(),
                format_hoon(hoon, config),
            ])
        }

        Hoon::TisFas(skin, value, body) => {
            // =/ - define
            rune3_backstep(
                TIS_FAS,
                format_skin(skin, config),
                format_hoon(value, config),
                format_hoon(body, config),
                config.indent_width as i32,
            )
        }

        Hoon::TisMic(skin, value, body) => {
            // =; - reversed define
            rune3_backstep(
                TIS_MIC,
                format_skin(skin, config),
                format_hoon(value, config),
                format_hoon(body, config),
                config.indent_width as i32,
            )
        }

        Hoon::TisDot(wing, value, body) => {
            // =. - change
            rune3_backstep(
                TIS_DOT,
                format_wing(wing, config),
                format_hoon(value, config),
                format_hoon(body, config),
                config.indent_width as i32,
            )
        }

        Hoon::TisWut(wing, test, yes, no) => {
            // =? - conditional change
            rune4_backstep(
                TIS_WUT,
                format_wing(wing, config),
                format_hoon(test, config),
                format_hoon(yes, config),
                format_hoon(no, config),
                config.indent_width as i32,
            )
        }

        Hoon::TisGal(a, b) => {
            // =< - compose reversed
            rune2_backstep(
                TIS_GAL,
                format_hoon(a, config),
                format_hoon(b, config),
                config.indent_width as i32,
            )
        }

        Hoon::TisHep(a, b) => {
            // =- - reversed pin
            rune2_backstep(
                TIS_HEP,
                format_hoon(a, config),
                format_hoon(b, config),
                config.indent_width as i32,
            )
        }

        Hoon::TisGar(a, b) => {
            // => - compose
            rune2_backstep(
                TIS_GAR,
                format_hoon(a, config),
                format_hoon(b, config),
                config.indent_width as i32,
            )
        }

        Hoon::TisKet(skin, wing, value, body) => {
            // =^ - pin from pair
            rune4_backstep(
                TIS_KET,
                format_skin(skin, config),
                format_wing(wing, config),
                format_hoon(value, config),
                format_hoon(body, config),
                config.indent_width as i32,
            )
        }

        Hoon::TisLus(value, body) => {
            // =+ - pin to head
            rune2_backstep(
                TIS_LUS,
                format_hoon(value, config),
                format_hoon(body, config),
                config.indent_width as i32,
            )
        }

        Hoon::TisSig(hoons) => {
            // =~ - compose list
            if hoons.len() == 1 {
                // Single expression - don't wrap in =~
                format_hoon(&hoons[0], config)
            } else {
                // Multiple expressions - use =~
                let children: Vec<Doc> = hoons.iter().map(|h| format_hoon(h, config)).collect();
                rune_vararg(TIS_SIG, children, config.indent_width as i32)
            }
        }

        Hoon::TisTar((name, spec), value, body) => {
            // =* - alias
            let name_doc = if let Some(s) = spec {
                Doc::concat(vec![
                    Doc::text(name.clone()),
                    Doc::text("/"),
                    format_spec(s, config),
                ])
            } else {
                Doc::text(name.clone())
            };
            rune3_backstep(
                TIS_TAR,
                name_doc,
                format_hoon(value, config),
                format_hoon(body, config),
                config.indent_width as i32,
            )
        }

        Hoon::TisCom(a, b) => {
            // =, - expose namespace
            rune2_backstep(
                TIS_COM,
                format_hoon(a, config),
                format_hoon(b, config),
                config.indent_width as i32,
            )
        }

        // ===== Wut ? Runes - Conditionals =====

        Hoon::WutBar(hoons) => {
            // ?| - logical OR
            let children: Vec<Doc> = hoons.iter().map(|h| format_hoon(h, config)).collect();
            rune_vararg(WUT_BAR, children, config.indent_width as i32)
        }

        Hoon::WutHep(wing, cases) => {
            // ?- - switch
            let wing_doc = format_wing(wing, config);
            let case_docs: Vec<Doc> = cases
                .iter()
                .map(|(spec, hoon)| {
                    Doc::concat(vec![
                        format_spec(spec, config),
                        Doc::gap(),
                        format_hoon(hoon, config),
                    ])
                })
                .collect();
            Doc::concat(vec![
                Doc::text(WUT_HEP),
                Doc::gap(),
                wing_doc,
                Doc::gap(),
                Doc::join(Doc::gap(), case_docs),
                Doc::gap(),
                Doc::text("=="),
            ])
        }

        Hoon::WutCol(test, yes, no) => {
            // ?: - if-then-else
            rune3_backstep(
                WUT_COL,
                format_hoon(test, config),
                format_hoon(yes, config),
                format_hoon(no, config),
                config.indent_width as i32,
            )
        }

        Hoon::WutDot(test, yes, no) => {
            // ?. - reversed if
            rune3_backstep(
                WUT_DOT,
                format_hoon(test, config),
                format_hoon(yes, config),
                format_hoon(no, config),
                config.indent_width as i32,
            )
        }

        Hoon::WutKet(wing, yes, no) => {
            // ?^ - if cell
            rune3_backstep(
                WUT_KET,
                format_wing(wing, config),
                format_hoon(yes, config),
                format_hoon(no, config),
                config.indent_width as i32,
            )
        }

        Hoon::WutGal(test, crash) => {
            // ?< - assert no
            tall_or_wide(
                WUT_GAL,
                vec![format_hoon(test, config), format_hoon(crash, config)],
            )
        }

        Hoon::WutGar(test, pass) => {
            // ?> - assert yes
            tall_or_wide(
                WUT_GAR,
                vec![format_hoon(test, config), format_hoon(pass, config)],
            )
        }

        Hoon::WutLus(wing, default, cases) => {
            // ?+ - switch with default
            let wing_doc = format_wing(wing, config);
            let default_doc = format_hoon(default, config);
            let case_docs: Vec<Doc> = cases
                .iter()
                .map(|(spec, hoon)| {
                    Doc::concat(vec![
                        format_spec(spec, config),
                        Doc::gap(),
                        format_hoon(hoon, config),
                    ])
                })
                .collect();
            Doc::concat(vec![
                Doc::text(WUT_LUS),
                Doc::gap(),
                wing_doc,
                Doc::gap(),
                default_doc,
                Doc::gap(),
                Doc::join(Doc::gap(), case_docs),
                Doc::gap(),
                Doc::text("=="),
            ])
        }

        Hoon::WutPam(hoons) => {
            // ?& - logical AND
            let children: Vec<Doc> = hoons.iter().map(|h| format_hoon(h, config)).collect();
            rune_vararg(WUT_PAM, children, config.indent_width as i32)
        }

        Hoon::WutPat(wing, yes, no) => {
            // ?@ - if atom
            rune3_backstep(
                WUT_PAT,
                format_wing(wing, config),
                format_hoon(yes, config),
                format_hoon(no, config),
                config.indent_width as i32,
            )
        }

        Hoon::WutSig(wing, yes, no) => {
            // ?~ - if null
            rune3_backstep(
                WUT_SIG,
                format_wing(wing, config),
                format_hoon(yes, config),
                format_hoon(no, config),
                config.indent_width as i32,
            )
        }

        Hoon::WutHax(skin, wing) => {
            // ?# - pattern match
            tall_or_wide(
                WUT_HAX,
                vec![format_skin(skin, config), format_wing(wing, config)],
            )
        }

        Hoon::WutTis(spec, wing) => {
            // ?= - type test
            tall_or_wide(
                WUT_TIS,
                vec![format_spec(spec, config), format_wing(wing, config)],
            )
        }

        Hoon::WutZap(hoon) => {
            // ?! - logical NOT
            Doc::concat(vec![Doc::text(WUT_ZAP), Doc::gap(), format_hoon(hoon, config)])
        }

        // ===== Zap ! Runes - Wild =====

        Hoon::ZapCom(a, b) => {
            // !, - normalize
            tall_or_wide(
                ZAP_COM,
                vec![format_hoon(a, config), format_hoon(b, config)],
            )
        }

        Hoon::ZapGar(hoon) => {
            // !> - type-value cell
            Doc::concat(vec![Doc::text("!>"), Doc::gap(), format_hoon(hoon, config)])
        }

        Hoon::ZapGal(spec, hoon) => {
            // !< - wrap type
            tall_or_wide(
                ZAP_GAL,
                vec![format_spec(spec, config), format_hoon(hoon, config)],
            )
        }

        Hoon::ZapMic(a, b) => {
            // !; - untyped
            tall_or_wide(
                ZAP_MIC,
                vec![format_hoon(a, config), format_hoon(b, config)],
            )
        }

        Hoon::ZapTis(hoon) => {
            // != - emit nock
            Doc::concat(vec![Doc::text(ZAP_TIS), Doc::gap(), format_hoon(hoon, config)])
        }

        Hoon::ZapPat(wings, yes, no) => {
            // !@ - virtual nock
            let wings_doc = Doc::join(
                Doc::text(" "),
                wings.iter().map(|w| format_wing(w, config)).collect(),
            );
            rune3_backstep(
                ZAP_PAT,
                wings_doc,
                format_hoon(yes, config),
                format_hoon(no, config),
                config.indent_width as i32,
            )
        }

        Hoon::ZapWut(arg, hoon) => {
            // !? - version check
            let arg_doc = match arg {
                ZpwtArg::ParsedAtom(s) => Doc::text(format!("%{}", s)),
                ZpwtArg::Pair(a, b) => Doc::text(format!("[%{} %{}]", a, b)),
            };
            rune2_backstep(
                ZAP_WUT,
                arg_doc,
                format_hoon(hoon, config),
                config.indent_width as i32,
            )
        }

        // Irregular syntax forms (preserved when preserve_syntax=true)
        Hoon::CellLiteral(hoons) => {
            // [a b c] - cell literal
            let children: Vec<Doc> = hoons.iter().map(|h| format_hoon(h, config)).collect();
            Doc::concat(vec![
                Doc::text("["),
                Doc::join(Doc::text(" "), children),
                Doc::text("]"),
            ])
        }

        Hoon::ListLiteral(hoons) => {
            // ~[a b c] - list literal
            let children: Vec<Doc> = hoons.iter().map(|h| format_hoon(h, config)).collect();
            Doc::concat(vec![
                Doc::text("~["),
                Doc::join(Doc::text(" "), children),
                Doc::text("]"),
            ])
        }

        Hoon::NameBinding(skin, hoon) => {
            // a=b - name binding
            Doc::concat(vec![
                format_skin(skin, config),
                Doc::text("="),
                format_hoon(hoon, config),
            ])
        }

        Hoon::FunctionCall(gate, sample) => {
            // (foo bar) - function call
            Doc::concat(vec![
                Doc::text("("),
                format_hoon(gate, config),
                Doc::text(" "),
                format_hoon(sample, config),
                Doc::text(")"),
            ])
        }
    }
}

// ===== Helper Functions =====

/// Format an axis number.
fn format_axis(n: u64) -> Doc {
    match n {
        0 => Doc::text("+0"),
        1 => Doc::text("."),
        2 => Doc::text("-"),
        3 => Doc::text("+"),
        _ => Doc::text(format!("+{}", n)),
    }
}

/// Format a Leaf constant.
fn format_leaf(aura: &str, value: &ParsedAtom, config: &FormatterConfig) -> Doc {
    match aura {
        "tas" => {
            let s = atom_to_cord(value);
            Doc::text(format!("%{}", s))
        }
        _ => value.format(config),
    }
}

/// Format a Rock constant.
fn format_rock(aura: &str, value: &NounExpr, config: &FormatterConfig) -> Doc {
    match aura {
        "f" => {
            // Loobean
            match value {
                NounExpr::ParsedAtom(ParsedAtom::Small(0)) => Doc::text("%.y"),
                NounExpr::ParsedAtom(ParsedAtom::Small(1)) => Doc::text("%.n"),
                _ => format_noun_expr(value, config),
            }
        }
        "n" => Doc::text("~"),
        "tas" => {
            // Term
            if let NounExpr::ParsedAtom(a) = value {
                let s = atom_to_cord(a);
                Doc::text(format!("%{}", s))
            } else {
                format_noun_expr(value, config)
            }
        }
        _ => format_noun_expr(value, config),
    }
}

/// Format a Sand constant.
fn format_sand(aura: &str, value: &NounExpr, config: &FormatterConfig) -> Doc {
    match aura {
        "f" => {
            // Loobean
            match value {
                NounExpr::ParsedAtom(ParsedAtom::Small(0)) => Doc::text("&"),
                NounExpr::ParsedAtom(ParsedAtom::Small(1)) => Doc::text("|"),
                _ => format_noun_expr(value, config),
            }
        }
        "t" => {
            // Cord
            if let NounExpr::ParsedAtom(a) = value {
                let s = atom_to_cord(a);
                Doc::text(format!("'{}'", escape_cord(&s)))
            } else {
                format_noun_expr(value, config)
            }
        }
        "ta" => {
            // Knot
            if let NounExpr::ParsedAtom(a) = value {
                let s = atom_to_cord(a);
                Doc::text(format!("~.{}", s))
            } else {
                format_noun_expr(value, config)
            }
        }
        "tas" => {
            // Term
            if let NounExpr::ParsedAtom(a) = value {
                let s = atom_to_cord(a);
                Doc::text(format!("%{}", s))
            } else {
                format_noun_expr(value, config)
            }
        }
        _ => format_noun_expr(value, config),
    }
}

/// Format a cord character in a string.
fn format_cord_char(a: &ParsedAtom) -> Doc {
    let c = match a {
        ParsedAtom::Small(n) => (*n & 0xFF) as u8 as char,
        ParsedAtom::Big(b) => {
            let bytes = b.to_bytes_le();
            if bytes.is_empty() {
                return Doc::nil();
            }
            bytes[0] as char
        }
    };
    match c {
        '"' => Doc::text("\\\""),
        '\\' => Doc::text("\\\\"),
        '\n' => Doc::text("\\n"),
        '\t' => Doc::text("\\t"),
        _ => Doc::text(c.to_string()),
    }
}

/// Convert an atom to a cord string.
fn atom_to_cord(value: &ParsedAtom) -> String {
    match value {
        ParsedAtom::Small(n) => {
            let mut s = String::new();
            let mut v = *n;
            while v > 0 {
                s.push((v & 0xFF) as u8 as char);
                v >>= 8;
            }
            s
        }
        ParsedAtom::Big(b) => {
            let bytes = b.to_bytes_le();
            String::from_utf8_lossy(&bytes).to_string()
        }
    }
}

/// Escape a cord for output.
fn escape_cord(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            '\'' => result.push_str("\\'"),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\t' => result.push_str("\\t"),
            _ => result.push(c),
        }
    }
    result
}

/// Format a core (|%, |@, etc.).
fn format_core(rune: &str, name: &Option<String>, arms: &IndexMap<String, Tome>, config: &FormatterConfig) -> Doc {
    let mut parts = vec![Doc::text(rune)];

    if let Some(n) = name {
        parts.push(Doc::gap());
        parts.push(Doc::text(format!("%{}", n)));
    }

    parts.push(format_arms(arms, config));
    parts.push(Doc::hardline());
    parts.push(Doc::text("--"));

    Doc::concat(parts)
}

/// Format a door (|_).
fn format_door(
    rune: &str,
    spec: &Spec,
    alas: &Alas,
    arms: &IndexMap<String, Tome>,
    config: &FormatterConfig,
) -> Doc {
    let mut parts = vec![Doc::text(rune), Doc::gap(), format_spec(spec, config)];

    // Format aliases
    for (name, hoon) in alas {
        parts.push(Doc::gap());
        parts.push(Doc::text(format!("{}=", name)));
        parts.push(format_hoon(hoon, config));
    }

    parts.push(format_arms(arms, config));
    parts.push(Doc::hardline());
    parts.push(Doc::text("--"));

    Doc::concat(parts)
}

/// Format core arms.
fn format_arms(arms: &IndexMap<String, Tome>, config: &FormatterConfig) -> Doc {
    let mut parts = Vec::new();

    for (chapter_name, (_what, arms_map)) in arms {
        if !chapter_name.is_empty() && chapter_name != "$" {
            parts.push(Doc::hardline());
            parts.push(Doc::text(format!("|%  {}", chapter_name)));
        }

        for (arm_name, hoon) in arms_map {
            parts.push(Doc::hardline());
            parts.push(Doc::text(format!("++  {}", arm_name)));
            parts.push(Doc::nest(
                config.indent_width as i32,
                Doc::concat(vec![Doc::hardline(), format_hoon(hoon, config)]),
            ));
        }
    }

    Doc::concat(parts)
}

/// Format a tyre (list of term-hoon pairs).
fn format_tyre(tyre: &Tyre, config: &FormatterConfig) -> Doc {
    if tyre.is_empty() {
        return Doc::text("~");
    }

    let pairs: Vec<Doc> = tyre
        .iter()
        .map(|(term, hoon)| {
            Doc::concat(vec![
                Doc::text(format!("%{}", term)),
                Doc::text("  "),
                format_hoon(hoon, config),
            ])
        })
        .collect();

    Doc::concat(vec![
        Doc::text("~["),
        Doc::join(Doc::gap(), pairs),
        Doc::text("]"),
    ])
}

/// Format a Manx (Sail XML element).
fn format_manx(manx: &Manx, config: &FormatterConfig) -> Doc {
    let tag = manx.g.n.format(config);

    // Format attributes
    let attrs: Vec<Doc> = manx
        .g
        .a
        .iter()
        .map(|(name, beers)| {
            let value = format_beers(beers, config);
            Doc::concat(vec![name.format(config), Doc::text("="), value])
        })
        .collect();

    let attrs_doc = if attrs.is_empty() {
        Doc::nil()
    } else {
        Doc::concat(vec![Doc::text(" "), Doc::join(Doc::text(" "), attrs)])
    };

    // Format children
    let children_doc = format_marl(&manx.c, config);

    if manx.c.is_empty() {
        Doc::concat(vec![Doc::text(";"), tag, attrs_doc, Doc::text(";")])
    } else {
        Doc::concat(vec![
            Doc::text(";"),
            tag,
            attrs_doc,
            Doc::nest(
                config.indent_width as i32,
                Doc::concat(vec![Doc::gap(), children_doc]),
            ),
            Doc::gap(),
            Doc::text("=="),
        ])
    }
}

/// Format a Marl (list of Tuna).
fn format_marl(marl: &Marl, config: &FormatterConfig) -> Doc {
    let parts: Vec<Doc> = marl.iter().map(|t| format_tuna(t, config)).collect();
    Doc::join(Doc::gap(), parts)
}

/// Format a Tuna.
fn format_tuna(tuna: &Tuna, config: &FormatterConfig) -> Doc {
    match tuna {
        Tuna::Manx(manx) => format_manx(manx, config),
        Tuna::TunaTail(tail) => match tail {
            TunaTail::Tape(hoon) => {
                Doc::concat(vec![Doc::text(";\""), format_hoon(hoon, config), Doc::text("\"")])
            }
            TunaTail::Manx(hoon) => {
                Doc::concat(vec![Doc::text(";+"), format_hoon(hoon, config)])
            }
            TunaTail::Marl(hoon) => {
                Doc::concat(vec![Doc::text(";*"), format_hoon(hoon, config)])
            }
            TunaTail::Call(hoon) => {
                Doc::concat(vec![Doc::text(";("), format_hoon(hoon, config), Doc::text(")")])
            }
        },
    }
}

/// Format beers (string with interpolation).
fn format_beers(beers: &[Beer], config: &FormatterConfig) -> Doc {
    let mut parts = vec![Doc::text("\"")];
    for beer in beers {
        match beer {
            Beer::Char(c) => {
                // Escape special chars
                let escaped = match c.as_str() {
                    "\"" => "\\\"",
                    "\\" => "\\\\",
                    "\n" => "\\n",
                    "\t" => "\\t",
                    s => s,
                };
                parts.push(Doc::text(escaped));
            }
            Beer::Hoon(h) => {
                parts.push(Doc::text("{"));
                parts.push(format_hoon(h, config));
                parts.push(Doc::text("}"));
            }
        }
    }
    parts.push(Doc::text("\""));
    Doc::concat(parts)
}
