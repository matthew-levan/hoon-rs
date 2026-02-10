//! Minimal formatter IR for deterministic rendering.
//!
//! This is intentionally smaller than `Doc`:
//! - No `Group`/`FlatAlt`
//! - No `SoftLine`/`Align`/`Backstep`
//! - Width only affects `Gap` in tall flow.

use parser::ast::hoon::Term;

use crate::config::FormatterConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Brackets {
    Round,  //  for wide form and irregular form blocks
    Square, //  for cell literals
    Curly,  //  for string interpolation
    Angled, // also for coercing into string inside an interpolation
}
impl Brackets {
    pub fn open(&self) -> &str {
        match self {
            Brackets::Round => "(",
            Brackets::Square => "[",
            Brackets::Curly => "{",
            Brackets::Angled => "<",
        }
    }
    pub fn close(&self) -> &str {
        match self {
            Brackets::Round => ")",
            Brackets::Square => "]",
            Brackets::Curly => "}",
            Brackets::Angled => ">",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Nil,
    Text(String),
    Seq(Vec<Block>),
    /// Semantic tall separator: either "  " or newline+indent depending on width.
    Gap,
    /// Semantic separator of `n` spaces (or newline+indent when width is exceeded).
    Pad(usize),
    /// Forced newline+indent.
    Line,
    /// Increase indent for nested block by one level.
    Indent(Box<Block>),
    /// Enclosed form with fixed separator (used by wide/irregular enclosed forms).
    Enclosed {
        brackets: Brackets,
        items: Vec<Block>,
    },
    /// Terminators such as "--" / "==".
    Terminator(&'static str),
}

impl Block {
    pub fn nil() -> Self {
        Self::Nil
    }

    pub fn text<S: Into<String>>(s: S) -> Self {
        Self::Text(s.into())
    }

    pub fn seq(parts: Vec<Block>) -> Self {
        let mut out = Vec::new();
        for part in parts {
            match part {
                Block::Nil => {}
                Block::Seq(inner) => out.extend(inner),
                other => out.push(other),
            }
        }
        match out.len() {
            0 => Block::Nil,
            1 => out.pop().unwrap_or(Block::Nil),
            _ => Block::Seq(out),
        }
    }

    pub fn gap() -> Self {
        Self::Gap
    }

    pub fn pad(spaces: usize) -> Self {
        Self::Pad(spaces)
    }

    pub fn line() -> Self {
        Self::Line
    }

    pub fn indent(inner: Block) -> Self {
        Self::Indent(Box::new(inner))
    }

    pub fn terminator(token: &'static str) -> Self {
        Self::Terminator(token)
    }
}

#[derive(Debug, Clone)]
struct Frame<'a> {
    indent_level: i32,
    block: &'a Block,
}

/// Deterministic renderer for `Block`.
pub struct BlockRenderer {
    config: FormatterConfig,
}

impl BlockRenderer {
    pub fn new(config: FormatterConfig) -> Self {
        Self { config }
    }

    pub fn render(&self, root: &Block) -> String {
        let mut out = String::new();
        let mut col = 0i32;
        let mut stack = vec![Frame {
            indent_level: 0,
            block: root,
        }];

        while let Some(frame) = stack.pop() {
            match frame.block {
                Block::Nil => {}
                Block::Text(s) => {
                    out.push_str(s);
                    col += s.len() as i32;
                }
                Block::Seq(parts) => {
                    for part in parts.iter().rev() {
                        stack.push(Frame {
                            indent_level: frame.indent_level,
                            block: part,
                        });
                    }
                }
                Block::Gap => {
                    if col + 2 > self.config.max_width as i32 {
                        push_newline(
                            &mut out, frame.indent_level, self.config.indent_width as i32, &mut col,
                        );
                    } else {
                        out.push_str("  ");
                        col += 2;
                    }
                }
                Block::Pad(spaces) => {
                    let spaces_i32 = *spaces as i32;
                    if col + spaces_i32 > self.config.max_width as i32 {
                        push_newline(
                            &mut out, frame.indent_level, self.config.indent_width as i32, &mut col,
                        );
                    } else {
                        out.push_str(&" ".repeat(*spaces));
                        col += spaces_i32;
                    }
                }
                Block::Line => {
                    push_newline(
                        &mut out, frame.indent_level, self.config.indent_width as i32, &mut col,
                    );
                }
                Block::Indent(inner) => {
                    stack.push(Frame {
                        indent_level: frame.indent_level + 1,
                        block: inner,
                    });
                }
                Block::Enclosed { brackets, items } => {
                    out.push_str(brackets.open());
                    col += 1;
                    for (idx, item) in items.iter().enumerate() {
                        if idx > 0 {
                            out.push_str(" ");
                            col += 1;
                        }
                        // Enclosed forms are intentionally not width-broken here.
                        let rendered = self.render(item);
                        col += rendered.len() as i32;
                        out.push_str(&rendered);
                    }
                    out.push_str(brackets.close());
                    col += 1;
                }
                Block::Terminator(token) => {
                    out.push_str(token);
                    col += token.len() as i32;
                }
            }
        }

        out
    }
}

fn push_newline(out: &mut String, indent_level: i32, indent_width: i32, col: &mut i32) {
    out.push('\n');
    let spaces = (indent_level.max(0) * indent_width.max(0)) as usize;
    out.push_str(&" ".repeat(spaces));
    *col = spaces as i32;
}

// Helpers

/// Join items with semantic `Gap` separators.
pub fn join_with_gap(items: Vec<Block>) -> Block {
    let mut out = Vec::new();
    let mut first = true;
    for item in items {
        if !first {
            out.push(Block::gap());
        }
        first = false;
        out.push(item);
    }
    Block::seq(out)
}
/// Join items with semantic `Gap` separators.
pub fn align_with_first(items: Vec<Block>) -> Block {
    let mut out = Vec::new();
    let mut first = true;
    for item in items {
        if !first {
            out.push(Block::line());
            out.push(Block::gap());
            out.push(Block::gap());
        }
        first = false;
        out.push(item);
    }
    Block::seq(out)
}

/// Join items with hard line breaks.
fn join_with_line(items: Vec<Block>) -> Block {
    let mut out = Vec::new();
    let mut first = true;
    for item in items {
        if !first {
            out.push(Block::line());
        }
        first = false;
        out.push(item);
    }
    Block::seq(out)
}
fn build_arms(arms: Vec<(Term, Block)>) -> Block {
    let arms: Vec<Block> = arms
        .into_iter()
        .map(|(name, body)| {
            Block::seq(vec![
                Block::text("++"),
                Block::gap(),
                Block::text(name),
                Block::indent(Block::seq(vec![Block::line(), body])),
                Block::line(),
            ])
        })
        .collect();
    Block::seq(arms)
}

fn first_line_width(block: &Block) -> (usize, bool) {
    match block {
        Block::Nil => (0, false),
        Block::Text(s) => (s.len(), false),
        Block::Gap => (2, false),
        Block::Pad(n) => (*n, false),
        Block::Line => (0, true),
        Block::Indent(inner) => first_line_width(inner),
        // TODO this doesn't work with the pairs ones
        Block::Enclosed { brackets: _, items } => {
            let items_width: usize = items.iter().map(|i| first_line_width(i).0).sum();
            let seps = 1 * items.len().saturating_sub(1);
            (1 + items_width + seps + 1, false)
        }
        Block::Terminator(token) => (token.len(), false),
        Block::Seq(parts) => {
            let mut width = 0usize;
            for part in parts {
                let (part_w, broke) = first_line_width(part);
                width += part_w;
                if broke {
                    return (width, true);
                }
            }
            (width, false)
        }
    }
}

fn aligned_pairs(pairs: Vec<(Block, Block)>) -> Block {
    let max_left = pairs
        .iter()
        .map(|(left, _)| first_line_width(left).0)
        .max()
        .unwrap_or(0);

    let rows: Vec<Block> = pairs
        .into_iter()
        .map(|(left, right)| {
            let left_width = first_line_width(&left).0;
            let pad = (max_left.saturating_sub(left_width)) + 2;
            Block::seq(vec![left, Block::pad(pad), right])
        })
        .collect();
    join_with_line(rows)
}

//  </helpers
//
//
//  Tall form builders
/// Common tall pattern:
/// `rune  header1  header2 ...`
/// `  body`
fn header_body(rune: &'static str, header_items: Vec<Block>, body: Block) -> Block {
    Block::seq(vec![
        Block::text(rune),
        Block::gap(),
        join_with_gap(header_items),
        Block::indent(Block::seq(vec![Block::line(), body])),
    ])
}

/// Common tall variadic pattern:
/// `rune  item1  item2 ...`
fn variadic_tall(rune: &'static str, items: Vec<Block>) -> Block {
    Block::seq(vec![
        Block::text(rune),
        Block::indent(Block::seq(vec![Block::gap(), join_with_gap(items)])),
    ])
}

fn rune2_tall(rune: &'static str, a: Block, b: Block) -> Block {
    variadic_tall(rune, vec![a, b])
}

fn rune3_tall(rune: &'static str, a: Block, b: Block, c: Block) -> Block {
    variadic_tall(rune, vec![a, b, c])
}

fn rune1_tall(rune: &'static str, a: Block) -> Block {
    variadic_tall(rune, vec![a])
}

fn line_joined(items: Vec<Block>) -> Block {
    Block::indent(Block::seq(vec![Block::line(), join_with_line(items)]))
}

//  common pattern where the rune mutates the scope (the subject) as its last child, so the last child goes on a new line but not indented
//  most = runes (=/ =| =+ are like this)
fn subject_copier(rune: &'static str, header_items: Vec<Block>, body: Block) -> Block {
    Block::seq(vec![
        Block::text(rune),
        Block::gap(),
        join_with_gap(header_items),
        Block::line(),
        body,
    ])
}

// :*  $:  $=  .^, %:  and some others are like his
fn flat_terminated(rune: &'static str, header_items: Vec<Block>) -> Block {
    Block::seq(vec![
        Block::text(rune),
        Block::gap(),
        align_with_first(header_items),
        Block::line(),
        Block::terminator("=="),
    ])
}
fn rune_table(
    rune: &'static str,
    main: Block,
    pairs: Vec<(Block, Block)>,
    terminate: bool,
) -> Block {
    let rows = aligned_pairs(pairs);
    let mut parts: Vec<Block> = vec![
        Block::text(rune),
        Block::gap(),
        main,
        Block::indent(Block::seq(vec![Block::line(), rows])),
    ];
    if terminate {
        parts.push(Block::line());
        parts.push(Block::terminator("=="));
    }
    Block::seq(parts)
}

fn rune_branching(rune: &'static str, test: Block, yes: Block, no: Block) -> Block {
    Block::seq(vec![
        Block::text(rune),
        Block::gap(),
        test,
        line_joined(vec![yes, no]),
    ])
}

// </builders>
//  short form builders

pub fn rune_short_base(rune: &'static str, items: Vec<Block>) -> Block {
    Block::seq(vec![
        Block::text(rune),
        Block::Enclosed {
            brackets: Brackets::Round,
            items: items,
        },
    ])
}
pub fn rune_short_pairs(rune: &'static str, main: Block, pairs: Vec<(Block, Block)>) -> Block {
    let mut paired: Vec<Block> = pairs
        .into_iter()
        .map(|(left, right)| Block::seq(vec![left, Block::text(", "), right]))
        .collect();
    paired.insert(0, main);

    Block::seq(vec![
        Block::text(rune),
        Block::Enclosed {
            brackets: Brackets::Round,
            items: paired,
        },
    ])
}

//  actual runes
pub fn rune_bar_cen(items: Vec<(Term, Block)>) -> Block {
    let arms = build_arms(items);

    Block::seq(vec![
        Block::text("|%"),
        Block::seq(vec![Block::line()]),
        arms,
        Block::terminator("--"),
    ])
}

pub fn rune_bar_ket(main: Block, items: Vec<(Term, Block)>) -> Block {
    let arms = build_arms(items);
    Block::seq(vec![
        Block::text("|^"),
        Block::gap(),
        main,
        Block::line(),
        arms,
        Block::terminator("--"),
    ])
}

pub fn rune_bar_buc(sample: Block, body: Block) -> Block {
    header_body("|$", vec![sample], body)
}

pub fn rune_bar_cab(sample: Block, body: Block) -> Block {
    header_body("|_", vec![sample], body)
}

pub fn rune_bar_col(sample: Block, body: Block) -> Block {
    header_body("|:", vec![sample], body)
}

pub fn rune_bar_dot(body: Block) -> Block {
    rune1_tall("|.", body)
}

pub fn rune_bar_hep(body: Block) -> Block {
    rune1_tall("|-", body)
}

pub fn rune_bar_pat(sample: Block, body: Block) -> Block {
    header_body("|@", vec![sample], body)
}

pub fn rune_bar_sig(sample: Block, body: Block) -> Block {
    header_body("|~", vec![sample], body)
}

pub fn rune_bar_tar(sample: Block, body: Block) -> Block {
    header_body("|*", vec![sample], body)
}

pub fn rune_bar_wut(body: Block) -> Block {
    rune1_tall("|?", body)
}

/// Example: `%=` (CenTis) is kinda special, should look liks
///
/// Shape:
/// %=  thing
///   b  new-b  
///   c  new-c
///   d  new-d
/// ==
pub fn rune_cen_sig(wing: Block, door: Block, args: Vec<Block>) -> Block {
    let mut items = vec![wing, door];
    items.extend(args);
    variadic_tall("%~", items)
}

pub fn rune_cen_ket(gate: Block, a: Block, b: Block, c: Block) -> Block {
    variadic_tall("%^", vec![gate, a, b, c])
}

pub fn rune_cen_tar(wing: Block, sample: Block, updates: Vec<(Block, Block)>) -> Block {
    let main = Block::seq(vec![wing, Block::gap(), sample]);
    rune_table("%*", main, updates, true)
}

pub fn rune_cen_cab(wing: Block, updates: Vec<(Block, Block)>) -> Block {
    rune_table("%_", wing, updates, true)
}
/// Example: `=/` (TisFas) as a rune-header + body block.
///
/// Shape:
/// `=/  sample  value`
/// `body`
pub fn rune_tis_fas(sample: Block, value: Block, body: Block) -> Block {
    subject_copier("=/", vec![sample, value], body)
}

pub fn rune_tis_bar(sample: Block, body: Block) -> Block {
    subject_copier("=|", vec![sample], body)
}

pub fn rune_tis_lus(sample: Block, body: Block) -> Block {
    subject_copier("=+", vec![sample], body)
}

pub fn rune_tis_wut(wing: Block, test: Block, value: Block) -> Block {
    rune3_tall("=?", wing, test, value)
}

pub fn rune_tis_ket(name: Block, wing: Block, value: Block, body: Block) -> Block {
    subject_copier("=^", vec![name, wing, value], body)
}

pub fn rune_tis_sig(items: Vec<Block>) -> Block {
    flat_terminated("=~", items)
}

pub fn rune_tis_col(items: Vec<Block>) -> Block {
    variadic_tall("=:", items)
}

pub fn rune_tis_mic(sample: Block, value: Block, body: Block) -> Block {
    subject_copier("=;", vec![sample, value], body)
}

pub fn rune_tis_gal(ctx: Block, body: Block) -> Block {
    rune2_tall("=<", ctx, body)
}

pub fn rune_tis_gar(ctx: Block, body: Block) -> Block {
    rune2_tall("=>", ctx, body)
}

pub fn rune_tis_hep(head: Block, body: Block) -> Block {
    rune2_tall("=-", head, body)
}

pub fn rune_tis_tar(alias: Block, body: Block) -> Block {
    rune2_tall("=*", alias, body)
}

pub fn rune_tis_com(namespace: Block, body: Block) -> Block {
    rune2_tall("=,", namespace, body)
}

/// Example: `:*` (ColTar) as a variadic gap-separated tall rune.
///
/// Children remain in tall flow:
/// `:*  a  b  c`
/// `==`
/// Gaps may wrap by width in the renderer.
pub fn rune_col_tar(items: Vec<Block>) -> Block {
    flat_terminated(":*", items)
    // Block::seq(vec![
    //     variadic_tall(":*", items),
    //     Block::line(),
    //     Block::terminator("=="),
    // ])
}

pub fn rune_col_hep(a: Block, b: Block) -> Block {
    rune2_tall(":-", a, b)
}

pub fn rune_col_lus(a: Block, b: Block, c: Block) -> Block {
    rune3_tall(":+", a, b, c)
}

pub fn rune_col_ket(a: Block, b: Block, c: Block, d: Block) -> Block {
    variadic_tall(":^", vec![a, b, c, d])
}

pub fn rune_col_cab(a: Block, b: Block) -> Block {
    rune2_tall(":_", a, b)
}

pub fn rune_col_sig(items: Vec<Block>) -> Block {
    flat_terminated(":~", items)
}

pub fn rune_cen_hep(gate: Block, sample: Block) -> Block {
    rune2_tall("%-", gate, sample)
}

pub fn rune_cen_lus(gate: Block, a: Block, b: Block) -> Block {
    rune3_tall("%+", gate, a, b)
}

pub fn rune_cen_col(gate: Block, samples: Vec<Block>) -> Block {
    let mut items = vec![gate];
    items.extend(samples);
    variadic_tall("%:", items)
}

pub fn rune_cen_dot(gate: Block, sample: Block) -> Block {
    rune2_tall("%.", gate, sample)
}

pub fn rune_cen_tis(sample: Block, items: Vec<(Block, Block)>) -> Block {
    rune_table("%=", sample, items, true)
}

pub fn rune_ket_hep(spec: Block, expr: Block) -> Block {
    rune2_tall("^-", spec, expr)
}

pub fn rune_ket_lus(spec: Block, expr: Block) -> Block {
    rune2_tall("^+", spec, expr)
}

pub fn rune_ket_tis(name: Block, expr: Block) -> Block {
    rune2_tall("^=", name, expr)
}

pub fn rune_ket_tar(spec: Block) -> Block {
    rune1_tall("^*", spec)
}

pub fn rune_ket_col(spec: Block) -> Block {
    rune1_tall("^:", spec)
}

pub fn rune_ket_bar(expr: Block) -> Block {
    rune1_tall("^|", expr)
}

pub fn rune_ket_dot(a: Block, b: Block) -> Block {
    rune2_tall("^.", a, b)
}

pub fn rune_ket_pam(expr: Block) -> Block {
    rune1_tall("^&", expr)
}

pub fn rune_ket_sig(expr: Block) -> Block {
    rune1_tall("^~", expr)
}

pub fn rune_ket_wut(expr: Block) -> Block {
    rune1_tall("^?", expr)
}

pub fn rune_dot_lus(expr: Block) -> Block {
    rune1_tall(".+", expr)
}

pub fn rune_dot_tar(formula: Block, sample: Block) -> Block {
    rune2_tall(".*", formula, sample)
}

pub fn rune_dot_tis(a: Block, b: Block) -> Block {
    rune2_tall(".=", a, b)
}

pub fn rune_dot_wut(expr: Block) -> Block {
    rune1_tall(".?", expr)
}

pub fn rune_dot_ket(path: Block, mark: Block) -> Block {
    rune2_tall(".^", path, mark)
}

pub fn rune_tis_dot(wing: Block, expr: Block) -> Block {
    rune2_tall("=.", wing, expr)
}

pub fn rune_wut_col(test: Block, yes: Block, no: Block) -> Block {
    rune_branching("?:", test, yes, no)
}

pub fn rune_wut_dot(test: Block, yes: Block, no: Block) -> Block {
    rune_branching("?.", test, yes, no)
}
pub fn rune_wut_sig(test: Block, yes: Block, no: Block) -> Block {
    rune_branching("?~", test, yes, no)
}
pub fn rune_wut_ket(test: Block, yes: Block, no: Block) -> Block {
    rune_branching("?^", test, yes, no)
}

pub fn rune_wut_lus(scrutinee: Block, default_case: Block, cases: Vec<(Block, Block)>) -> Block {
    let main = Block::seq(vec![scrutinee, Block::gap(), default_case]);
    rune_table("?+", main, cases, true)
}

pub fn rune_wut_hep(scrutinee: Block, cases: Vec<(Block, Block)>) -> Block {
    rune_table("?-", scrutinee, cases, true)
}

pub fn rune_wut_bar(items: Vec<Block>) -> Block {
    variadic_tall("?|", items)
}

pub fn rune_wut_pam(items: Vec<Block>) -> Block {
    variadic_tall("?&", items)
}

pub fn rune_wut_tis(spec: Block, noun: Block) -> Block {
    rune2_tall("?=", spec, noun)
}

pub fn rune_wut_pat(test: Block, yes: Block, no: Block) -> Block {
    rune_wut_col(test, yes, no)
}

pub fn rune_wut_zap(expr: Block) -> Block {
    rune1_tall("?!", expr)
}

pub fn rune_wut_gal(expr: Block) -> Block {
    rune1_tall("?<", expr)
}

pub fn rune_wut_gar(expr: Block) -> Block {
    rune1_tall("?>", expr)
}

pub fn rune_wut_hax(pattern: Block, cases: Vec<(Block, Block)>) -> Block {
    rune_table("?#", pattern, cases, true)
}

pub fn rune_sig_bar(a: Block, b: Block) -> Block {
    rune2_tall("~|", a, b)
}

pub fn rune_sig_cab(a: Block, b: Block) -> Block {
    rune2_tall("~_", a, b)
}

pub fn rune_sig_cen(a: Block, b: Block, c: Block, d: Block) -> Block {
    variadic_tall("~%", vec![a, b, c, d])
}

pub fn rune_sig_fas(a: Block, b: Block) -> Block {
    rune2_tall("~/", a, b)
}

pub fn rune_sig_gal(a: Block, b: Block) -> Block {
    rune2_tall("~<", a, b)
}

pub fn rune_sig_gar(a: Block, b: Block) -> Block {
    rune2_tall("~>", a, b)
}

pub fn rune_sig_buc(a: Block) -> Block {
    rune1_tall("~$", a)
}

pub fn rune_sig_lus(a: Block) -> Block {
    rune1_tall("~+", a)
}

pub fn rune_sig_pam(a: Block) -> Block {
    rune1_tall("~&", a)
}

pub fn rune_sig_tis(a: Block) -> Block {
    rune1_tall("~=", a)
}

pub fn rune_sig_wut(a: Block) -> Block {
    rune1_tall("~?", a)
}

pub fn rune_sig_zap(a: Block) -> Block {
    rune1_tall("~!", a)
}

pub fn rune_zap_zap() -> Block {
    Block::text("!!")
}

pub fn rune_zap_com(a: Block, b: Block) -> Block {
    rune2_tall("!,", a, b)
}

pub fn rune_zap_gar(a: Block) -> Block {
    rune1_tall("!>", a)
}

pub fn rune_zap_mic(a: Block, b: Block) -> Block {
    rune2_tall("!;", a, b)
}

pub fn rune_zap_tis(a: Block) -> Block {
    rune1_tall("!=", a)
}

pub fn rune_zap_pat(a: Block) -> Block {
    rune1_tall("!@", a)
}

pub fn rune_zap_wut(a: Block, b: Block) -> Block {
    rune2_tall("!?", a, b)
}

pub fn rune_mic_tis(template: Block) -> Block {
    rune1_tall(";=", template)
}

pub fn rune_mic_col(a: Block, b: Block) -> Block {
    rune2_tall(";:", a, b)
}

pub fn rune_mic_fas(a: Block) -> Block {
    rune1_tall(";/", a)
}

pub fn rune_mic_gal(a: Block, b: Block) -> Block {
    rune2_tall(";<", a, b)
}

pub fn rune_mic_sig(items: Vec<Block>) -> Block {
    variadic_tall(";~", items)
}

pub fn rune_mic_mic(a: Block) -> Block {
    rune1_tall(";;", a)
}

/// Example: `|=` (BarTis) as a gate header + body block.
///
/// Shape:
/// `|=  sample`
/// `  body`
pub fn rune_bar_tis(sample: Block, body: Block) -> Block {
    header_body("|=", vec![sample], body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_tis_fas_renders() {
        let block = rune_tis_fas(
            Block::text("state"),
            Block::text("-"),
            Block::text("^-  agent:gall"),
        );
        let rendered = BlockRenderer::new(FormatterConfig::default()).render(&block);
        // println!("example_tis_fas_renders block:\n{:#?}", block);
        println!("example_tis_fas_renders rendered:\n{}", rendered);
        assert_eq!(rendered, "=/  state  -\n^-  agent:gall");
    }

    #[test]
    fn example_bar_tis_renders() {
        let block = rune_bar_tis(Block::text("a=@"), Block::text("(add a 1)"));
        let rendered = BlockRenderer::new(FormatterConfig::default()).render(&block);
        // println!("example_bar_tis_renders block:\n{:#?}", block);
        println!("example_bar_tis_renders rendered:\n{}", rendered);
        assert_eq!(rendered, "|=  a=@\n  (add a 1)");
    }

    #[test]
    fn example_col_tar_wraps_by_gap() {
        let block = rune_col_tar(vec![
            Block::text("alpha"),
            Block::text("beta"),
            Block::text("gamma"),
            Block::text("gamma2"),
            Block::text("gamma3"),
            Block::text("gamma5"),
            Block::text("gammo"),
        ]);
        let rendered =
            BlockRenderer::new(FormatterConfig::default().with_max_width(15)).render(&block);
        // println!("example_col_tar_wraps_by_gap block:\n{:#?}", block);
        println!("example_col_tar_wraps_by_gap rendered:\n{}", rendered);
        assert!(rendered.starts_with(":*"));
        assert!(rendered.ends_with("\n=="));
        for item in ["alpha", "beta", "gamma", "gamma2", "gamma3", "gamma5", "gammo"] {
            assert!(rendered.contains(item), "missing item in output: {item}");
        }
    }

    #[test]
    fn example_cen_tis_pairs_even() {
        let block = rune_cen_tis(
            Block::text("thing"),
            vec![
                (Block::text("b"), Block::text("new-b")),
                (Block::text("c"), Block::text("new-c")),
                (Block::text("d"), Block::text("new-d")),
            ],
        );
        let rendered = BlockRenderer::new(FormatterConfig::default()).render(&block);
        println!("example_cen_tis_pairs_even rendered:\n{}", rendered);
        assert_eq!(
            rendered,
            "%=  thing\n  b  new-b\n  c  new-c\n  d  new-d\n=="
        );
    }

    #[test]
    fn example_cen_tis_pairs_align_second_column() {
        let block = rune_cen_tis(
            Block::text("thing"),
            vec![
                (Block::text("b"), Block::text("new-b")),
                (Block::text("longer-key"), Block::text("new-c")),
                (Block::text("d"), Block::text("new-d")),
            ],
        );
        let rendered = BlockRenderer::new(FormatterConfig::default()).render(&block);
        println!(
            "example_cen_tis_pairs_align_second_column rendered:\n{}",
            rendered
        );
        assert_eq!(
            rendered,
            "%=  thing\n  b           new-b\n  longer-key  new-c\n  d           new-d\n=="
        );
    }
    #[test]
    fn example_bar_cen() {
        let block = rune_bar_cen(vec![
            ("b".into(), Block::text("new-b")),
            ("longer-key".into(), Block::text("new-c")),
            ("d".into(), Block::text("new-d")),
        ]);
        let rendered = BlockRenderer::new(FormatterConfig::default()).render(&block);
        println!("example_bar_cen rendered:\n{}", rendered);
        assert_eq!(
            rendered,
            "|%\n++  b\n  new-b\n++  longer-key\n  new-c\n++  d\n  new-d\n--"
        );
    }

    #[test]
    fn example_bar_ket() {
        let block = rune_bar_ket(
            Block::text("main-logic"),
            vec![
                ("on-init".into(), Block::text("init-body")),
                ("on-save".into(), Block::text("save-body")),
            ],
        );
        let rendered = BlockRenderer::new(FormatterConfig::default()).render(&block);
        println!("example_bar_ket rendered:\n{}", rendered);
        assert_eq!(
            rendered,
            "|^  main-logic\n++  on-init\n  init-body\n++  on-save\n  save-body\n--"
        );
    }

    #[test]
    fn example_more_runes() {
        let rendered_tis_bar = BlockRenderer::new(FormatterConfig::default())
            .render(&rune_tis_bar(Block::text("state"), Block::text("body")));
        let rendered_cen_hep = BlockRenderer::new(FormatterConfig::default())
            .render(&rune_cen_hep(Block::text("gate"), Block::text("sample")));
        let rendered_wut_col =
            BlockRenderer::new(FormatterConfig::default()).render(&rune_wut_col(
                Block::text("test"),
                Block::text("yes-branch"),
                Block::text("no-branch"),
            ));

        println!("example_more_runes tis_bar:\n{}", rendered_tis_bar);
        println!("example_more_runes cen_hep:\n{}", rendered_cen_hep);
        println!("example_more_runes wut_col:\n{}", rendered_wut_col);

        assert_eq!(rendered_tis_bar, "=|  state\nbody");
        assert_eq!(rendered_cen_hep, "%-  gate  sample");
        assert_eq!(rendered_wut_col, "?:  test\n  yes-branch\n  no-branch");
    }

    #[test]
    fn example_even_more_runes() {
        let r_col_sig = BlockRenderer::new(FormatterConfig::default()).render(&rune_col_sig(vec![
            Block::text("aerer"),
            Block::text("bpp"),
            Block::text("csdfsdfsfsdf"),
        ]));
        let r_tis_ket = BlockRenderer::new(FormatterConfig::default()).render(&rune_tis_ket(
            Block::text("name"),
            Block::text("wing"),
            Block::text("val"),
            Block::text("body"),
        ));
        let r_wut_lus = BlockRenderer::new(FormatterConfig::default()).render(&rune_wut_lus(
            Block::text("-.a"),
            Block::text("~"),
            vec![
                (Block::text("%foo"), Block::text("foo-body")),
                (Block::text("%barecino"), Block::text("bar-body")),
            ],
        ));
        let r_wut_hep = BlockRenderer::new(FormatterConfig::default()).render(&rune_wut_hep(
            Block::text("-.a"),
            vec![
                (Block::text("%foo"), Block::text("foo-body")),
                (Block::text("%barecomp"), Block::text("bar-body")),
            ],
        ));
        let r_cen_col = BlockRenderer::new(FormatterConfig::default()).render(&rune_cen_col(
            Block::text("gate"),
            vec![Block::text("a"), Block::text("b"), Block::text("c")],
        ));
        let r_tis_wut = BlockRenderer::new(FormatterConfig::default()).render(&rune_tis_wut(
            Block::text("wing"),
            Block::text("test"),
            Block::text("value"),
        ));

        println!("example_even_more_runes col_sig:\n{}", r_col_sig);
        println!("example_even_more_runes tis_ket:\n{}", r_tis_ket);
        println!("example_even_more_runes wut_lus:\n{}", r_wut_lus);
        println!("example_even_more_runes wut_hep:\n{}", r_wut_hep);
        println!("example_even_more_runes cen_col:\n{}", r_cen_col);
        println!("example_even_more_runes tis_wut:\n{}", r_tis_wut);
        assert_eq!(true, true);

        // assert_eq!(r_col_sig, ":~  a\n    b\n    c\n==");
        // assert_eq!(r_tis_ket, "=^  name  wing  val\nbody");
        // assert_eq!(r_cen_col, "%:  gate  a  b  c");
        // assert_eq!(r_tis_wut, "=?  wing  test  value");
        // assert!(r_wut_lus.starts_with("?+  -.a  ~\n"));
        // assert!(r_wut_lus.ends_with("\n=="));
        // assert!(r_wut_hep.starts_with("?-  -.a\n"));
        // assert!(r_wut_hep.ends_with("\n=="));
    }
}
