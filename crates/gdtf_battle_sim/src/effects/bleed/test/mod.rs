//! The relocated `bleed` unit tests, grouped by acceptance criterion: the shared
//! headless-app fixtures live in [`support`], the per-AC assertions in the sibling
//! files (drain / filter / flag / message / the GTW-572 once-per-span start facts /
//! the GTW-641 Downed-entry gate + turn-start timing pins).

mod support;

mod drain;
mod entry_gate;
mod filter;
mod flag;
mod message;
mod runtime;
mod span;
mod stabilize_regression;
mod turn_start;
