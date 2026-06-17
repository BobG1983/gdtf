//! The relocated `bleed` unit tests, grouped by acceptance criterion: the shared
//! headless-app fixtures live in [`support`], the per-AC assertions in the sibling
//! files (drain / filter / flag / message).

mod support;

mod drain;
mod filter;
mod flag;
mod message;
