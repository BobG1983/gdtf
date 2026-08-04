//! Guard: integration tests under crate `tests/` must be dir-form (suite/main.rs),
//! not new flat `tests/*.rs` binaries — packing cut Bevy link cost (GTW-797 / GTW-835).

mod check;
mod tree;
