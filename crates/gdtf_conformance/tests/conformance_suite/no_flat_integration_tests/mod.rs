//! Guard: integration tests under `crates/*/tests/`, `bins/*/tests/` and `libs/*/tests/`
//! are one `<name>_suite/main.rs` binary per crate, never flat `tests/*.rs` binaries.

mod check;
mod tree;
