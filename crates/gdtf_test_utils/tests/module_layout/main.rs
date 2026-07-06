//! GTW-583 clause-7 module-layout conformance guard — enforces
//! `.claude/rules/module-layout.md` on every `cargo test --workspace` run.
//!
//! Walks the tracked `.rs` files under `crates/` + `bins/` (via `git ls-files`,
//! with a std fs-walk fallback when git is unavailable) and classifies each
//! file with the pinned census precedence: `mod.rs` basename first, then
//! `crates/<crate>/tests/` (integration), then in-src test patterns
//! (`test.rs`/`tests.rs`, `test/`/`tests/` dirs, `test_*` basenames,
//! `test_support`), else logic. Then it:
//!
//! - FAILS on any >400-line file in any band (`BLOCK`), unless the path is
//!   registered in `.claude/rules/module-layout-exemptions.txt`; a registered
//!   `lib.rs`/`main.rs` crate root must additionally REMAIN pure wiring (zero
//!   `fn`/`impl` after comment stripping) or it still fails;
//! - FAILS on any logic-bearing `mod.rs` (`MODLOGIC`): after comment
//!   stripping, ANY `fn` (the allowlist is EMPTY — user ruling 2026-07-04),
//!   any `impl` other than `Plugin for`/`PluginGroup for`, or a closure system
//!   inside `add_systems(`;
//! - FAILS on STALE registry entries (path missing, or no longer violating) —
//!   the registry only ever shrinks or is deliberately re-approved;
//! - WARNS (`eprintln!`, non-fatal) on 301-400-line files.
//!
//! Deterministic output — one violation per line, ordered (-lines, path) — and
//! a single final assert carrying the full list. Std-only, zero cargo
//! invocations. The workspace root defaults to `CARGO_MANIFEST_DIR/../..` and
//! can be overridden via the `GDTF_MODULE_LAYOUT_ROOT` env var (how the
//! GTW-583 pre-slice spot-check points the built guard at another checkout).

mod census;
mod conformance;
mod tree;
