//! Guard: every backticked `crates/`, `libs/` or `bins/` Rust path cited in `docs/`
//! or `.claude/` names a file that is on disk.

mod check;
mod tree;
