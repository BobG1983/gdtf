---
paths: ["**/*.rs"]
---

# Module layout

> **You MUST read and follow [plain-language.md](./plain-language.md) before writing anything here. It is
> binding on every word, and it is not optional.**

Why this rule exists: GTW-583's census found 117 files past the 400-line block line and 4
mod.rs files carrying logic. One big file mixes unrelated reasons to change, so edits collide,
review and bisect get harder, and no one reads it in one sitting.

## Rules

1. A module is a directory. A concern that outgrows one file gets `<name>/mod.rs` plus a
   submodule per concern. Split by what makes code change together. Never split by cutting a
   file in half.
2. mod.rs is wiring only: the module doc, `mod` declarations, and re-exports (`pub use` that
   preserves the module's public paths), and nothing else. No function of any kind: not a
   helper, not a `register_*(app: &mut App)` aggregator, not even a delegating
   `Plugin::build`. No impls, no type definitions, no closure systems inside `add_systems`.
3. The line limits cover every file: logic, unit tests and integration tests. Over 300
   lines, split now if the split is obvious. Over 400 is the block line: it must not land.
4. A `lib.rs` is wiring only on the same terms, plus `pub mod` declarations. An `src/main.rs`
   contains only `fn main()` and delegates everything else to crates.
5. Unit tests live in `<module>/test/mod.rs` behind `#[cfg(test)] mod test`. For integration
   tests, `<crate>/tests/main.rs` holds the suite module, which lives in
   `<crate>/tests/<suite_name>`.
6. A split is a pure move that preserves behaviour. Beyond the move you may change only
   visibility (`pub(super)`, `pub(crate)`, `pub(in path)`), import paths, and intra-doc links
   pointing at the new paths. Never invent a shared abstraction to shrink a line count. Never
   move logic out of `gdtf_battle_sim` into `gdtf_battle_presenter`, or back the other way. A
   helper used by two or more modules lives in the shared support or harness module; a helper
   with one consumer stays with that consumer.
7. A module's `pub use` may re-export only its own descendants' items, never a sibling's, a
   cousin's, or another family's (for example `pub use crate::other_family::…` presented as
   this module's API). Consumers then import the type through a path that misstates where it
   lives, and the two families read as one.

## Enforcement

The conformance test `crates/gdtf_test_utils/tests/module_layout/` walks the tracked tree on
every `cargo dtest` run. Any file over the block line, or any mod.rs carrying logic, fails the
suite.
