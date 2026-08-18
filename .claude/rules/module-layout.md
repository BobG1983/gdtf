---
paths: ["**/*.rs"]
---

# Module layout — a module is a DIRECTORY; mod.rs is wiring-only; files stay small

Why this rule exists: GTW-583's census found 117 files past the 400-line block
line and 4 logic-bearing mod.rs files. A monolith file lumps unrelated
change-reasons, so every edit collides with every other and review/bisect decay.
One concern per file, wiring separated from logic, keeps each change small and
each file readable in one sitting.

## Rules

1. **A module is a DIRECTORY.** A concern that outgrows one file becomes a
   directory module: `<name>/mod.rs` plus focused per-concern submodules. Split
   by CHANGE-REASON/CONCERN — "what makes this code change together" — never by
   mechanical halving.
2. **mod.rs is WIRING-ONLY — a mod.rs contains NO fns.** Permitted contents:
   the module doc, `mod` declarations, and re-exports (`pub use` preserving the
   module's public paths). NOTHING else: no fn of ANY kind — not a helper, not
   a `register_*(app: &mut App)` aggregator, not even a delegating
   `Plugin::build`, no impl, no type definitions, no closure systems inside
   `add_systems`.
3. **Line length bands cover EVERY FILE** logic, unit tests and integration tests. 
   **OK under 300** — no action.
   **AVOID over 300** — split now if the split is obvious. 
   **BLOCK over 400** — must not land; the conformance test will fail.
4. **Crate roots** (`lib.rs`/`src/main.rs`) follow the same rules. 
   A crate root (`lib.rs`) is PURE WIRING (docs + `mod`/`pub mod` decls + re-exports, 
   zero fn/impl). A binary root (`src/main.rs`) contains only `fn main()` and
   delegates everything else to crates.
5. **Test placement** follows the convention: sibling `test/`
   directory for unit tests; `<module>/tests/mod.rs` behind `#[cfg(test)] mod test. 
   For integration tests; `<crate>/tests/main.rs` contains the integration test 
   module, the module lives in `<crate>/tests/<suite_name>`.
6. **Splits are behavior-preserving pure moves.** The only edits allowed
   beyond the move: visibility (`pub(super)`/`pub(crate)`/`pub(in path)`),
   import paths, and intra-doc-link re-pathing.
   Never invent shared abstractions to shrink counts; never move logic out of
   `gdtf_battle_sim` into `gdtf_battle_presenter` or back the other way;
   helpers with 2+ consuming modules live in the shared support/harness
   module, single-consumer helpers stay local to their consumer.
7. **A module's `pub use` may lift only from its own DESCENDANTS.** A module
   re-exports its own submodules' items — never a sibling's, a cousin's, or
   another family's (`pub use crate::other_family::…` presented as this
   module's API). A cross-family re-export erases a concern split at the
   public surface: consumers import the type via the lying path and the
   families read as one in error.

## Enforcement

Conformance test: `crates/gdtf_test_utils/tests/module_layout/`
walks the tracked tree on every `cargo dtest` run — any overly large file or
logic-bearing mod.rs fails the suite, loudly. 
