//! GTW-929 rustdoc-gate guard — a rustdoc warning must be able to FAIL the two
//! doc steps, and the content-editor doc collision must stay fixed.
//!
//! The gap this pins: `[workspace.lints.rustdoc]` declared only
//! `broken_intra_doc_links` and `private_intra_doc_links` as `deny`. Every other
//! rustdoc lint stayed at its default `warn`, and no `RUSTDOCFLAGS` promoted
//! them anywhere, so `cargo doc --workspace --no-deps` and `cargo doc-full` both
//! exited 0 no matter how many rustdoc warnings accumulated. The count drifted
//! upward unobserved across three measurements (24, then 32, then 37
//! `redundant_explicit_links`) precisely because nothing could fail on it.
//! `all = { level = "deny", priority = -1 }` makes the whole group fatal.
//!
//! Three things have to hold together for that deny to actually bite, and each
//! is a separate test here:
//!
//! 1. The group deny is declared, with `priority = -1` so an individual lint
//!    listed below it can still override the group level.
//! 2. EVERY workspace member declares `[lints] workspace = true`. A
//!    `[workspace.lints]` table reaches only the crates that opt in, so one
//!    member missing that declaration is a hole the group deny never covers —
//!    and nothing else in the repo would notice.
//! 3. `bins/gdtf_content_editor`'s `[[bin]]` keeps `doc = false`. That bin
//!    target and the library crate `crates/gdtf_content_editor` share a name, so
//!    without it both write `target/doc/gdtf_content_editor/index.html`, cargo
//!    warns about the output filename collision, and one silently replaces the
//!    other's rendered page (rust-lang/cargo#6313). That warning comes from
//!    CARGO, not rustdoc, so no lint level reaches it — only the exclusion does,
//!    which is why it needs its own pin.
//!
//! Why a guard at all, when the doc runs are already in the suite: the doc runs
//! catch a rustdoc warning only while the configuration is intact. Deleting the
//! group deny, or dropping a member's `[lints] workspace = true`, leaves every
//! step of the suite green while quietly restoring the defect — the same
//! invisible-regression shape the sibling `ci_workflow_features` guard exists
//! for. The collision pin is the same story: with `doc = false` gone the doc
//! runs still exit 0, because a cargo warning is not a lint.
//!
//! Reads the manifests as text — no `toml` dependency for one guard, and no
//! cargo invocation — the same std-only shape as the sibling
//! `binary_feature_passthrough`, `ci_workflow_features`, `module_layout` and
//! `docs_path_truth` suites, whose tree enumeration and single-final-assert
//! reporting this suite follows. Paths stay plain `String`s here for the same
//! reason they do there: they are file paths in a guard's own scratch
//! enumeration, not domain values the game models.
//!
//! The repo root defaults to `CARGO_MANIFEST_DIR/../..` and can be overridden
//! via the `GDTF_RUSTDOC_GATE_ROOT` env var (the sibling guards' recipe, for
//! pointing the built guard at another checkout).

mod check;
mod manifest;
mod tree;
