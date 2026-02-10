//! Formatting for Hoon AST nodes.
//!
//! The dispatcher is split into rune-family modules to keep maintenance tractable.

mod bar;
mod basic;
mod cen;
mod col;
mod dot;
mod helpers;
mod irregular;
mod ket;
mod mic;
mod misc;
mod sig;
mod tis;
mod wut;
mod zap;

use parser::ast::hoon::{Hoon, Spec};

use crate::config::FormatterConfig;
use crate::doc::Doc;
use crate::format::mode::FormatMode;
use crate::format::spec::{format_spec_impl, format_spec_wide};

#[derive(Clone, Copy)]
pub(super) struct FormatCtx<'a> {
    pub(super) config: &'a FormatterConfig,
    pub(super) mode: FormatMode,
}

impl<'a> FormatCtx<'a> {
    pub(super) fn indent(self) -> i32 {
        self.config.indent_width as i32
    }

    pub(super) fn fmt(self, hoon: &Hoon) -> Doc {
        format_hoon_impl(hoon, self.config, self.mode)
    }

    pub(super) fn fmt_wide(self, hoon: &Hoon) -> Doc {
        format_hoon_impl(hoon, self.config, FormatMode::Wide)
    }

    pub(super) fn fmt_spec(self, spec: &Spec) -> Doc {
        format_spec_impl(spec, self.config, self.mode)
    }

    pub(super) fn fmt_spec_wide(self, spec: &Spec) -> Doc {
        format_spec_wide(spec, self.config)
    }
}

/// Format a Hoon AST node to a Doc (tall mode - default for top-level).
pub fn format_hoon(hoon: &Hoon, config: &FormatterConfig) -> Doc {
    format_hoon_impl(hoon, config, FormatMode::Tall)
}

/// Format a Hoon AST node in wide mode (for use inside () and []).
pub fn format_hoon_wide(hoon: &Hoon, config: &FormatterConfig) -> Doc {
    format_hoon_impl(hoon, config, FormatMode::Wide)
}

fn format_hoon_impl(hoon: &Hoon, config: &FormatterConfig, mode: FormatMode) -> Doc {
    let ctx = FormatCtx { config, mode };

    basic::format(hoon, ctx)
        .or_else(|| irregular::format(hoon, ctx))
        .or_else(|| bar::format(hoon, ctx))
        .or_else(|| col::format(hoon, ctx))
        .or_else(|| cen::format(hoon, ctx))
        .or_else(|| dot::format(hoon, ctx))
        .or_else(|| ket::format(hoon, ctx))
        .or_else(|| sig::format(hoon, ctx))
        .or_else(|| mic::format(hoon, ctx))
        .or_else(|| tis::format(hoon, ctx))
        .or_else(|| wut::format(hoon, ctx))
        .or_else(|| zap::format(hoon, ctx))
        .or_else(|| misc::format(hoon, ctx))
        .unwrap_or_else(|| panic!("unhandled hoon variant in formatter: {:?}", hoon))
}
