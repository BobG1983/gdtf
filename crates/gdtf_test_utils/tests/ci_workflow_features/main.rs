//! GTW-883 CI-workflow feature guard — the workspace-wide `cargo` commands in
//! `.github/workflows/` must name BOTH `net_qa` features, so an editor QA
//! regression fails a PR instead of compiling nowhere.
//!
//! The gap this pins: `test.yml` ran a bare `cargo test --workspace` and
//! `clippy.yml` a bare `cargo clippy --workspace --all-targets`. A
//! package-qualified feature turns on only that package's, so with neither name
//! present the editor's whole `crates/gdtf_content_editor/src/net_qa/` module
//! went unlinted and its two `#![cfg(all(debug_assertions, feature =
//! "net_qa"))]` test binaries reported "0 tests" on every PR. Deleting the
//! `--features` flag again is invisible to every other test in the repo: no
//! other test reads `.github/`, and the local `.cargo/config.toml` aliases
//! (which carry their own feature list) keep passing.
//!
//! The check is DERIVED, not a hand-kept list of workflow files: every tracked
//! `.github/workflows/*.yml` is read, every `run:` step invoking `cargo` with
//! `--workspace` is collected — in the single-line form and in the block form
//! (`run: |`, backslash continuations joined; see `run_steps.rs`) — and each
//! one must name both features. On top of that walk, `REQUIRED_COMMANDS` in
//! `check.rs` names the two commands that must have been reached, so a
//! regression in the reader cannot quietly make the guard vacuous by matching
//! nothing.
//!
//! CI green stays STATIC per `CLAUDE.md`, so the guard also rejects
//! `dynamic_linking` appearing on a workflow command.
//!
//! Reads the YAML as text — no `serde_yaml` dependency for one guard, and no
//! cargo invocation — the same std-only shape as the sibling
//! `binary_feature_passthrough`, `module_layout` and `docs_path_truth` suites,
//! whose tree enumeration and single-final-assert reporting this suite follows.
//! Paths stay plain `String`s here for the same reason they do there: they are
//! file paths in a guard's own scratch enumeration, not domain values the game
//! models.
//!
//! The repo root defaults to `CARGO_MANIFEST_DIR/../..` and can be overridden
//! via the `GDTF_CI_WORKFLOW_ROOT` env var (the sibling guards' recipe, for
//! pointing the built guard at another checkout).

mod check;
mod run_steps;
mod tree;
