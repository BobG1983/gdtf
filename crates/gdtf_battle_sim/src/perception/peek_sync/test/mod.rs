//! Pure unit tests for the GTW-406 positional peek populator — the [`corner_lean`]
//! geometry over hand-built grids (no `App`, no ECS — the sim-unit-test idiom). The
//! system-level producer→consumer bridge is the App-driven integration test
//! `tests/peek_offset_populator.rs`.
//!
//! [`corner_lean`]: super::corner::corner_lean

mod corner;
