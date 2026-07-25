//! GTW-878 binary-feature-passthrough guard — a binary package must expose the
//! `net_qa` QA control channel its own library offers, so the channel can never
//! again be reachable only from a `--features <lib>/net_qa` workspace check and
//! not from a launchable binary.
//!
//! The gap this pins: `bins/gdtf_content_editor/Cargo.toml` declared
//! `dynamic_linking` and `file_watcher` but no `net_qa`, while
//! `crates/gdtf_content_editor/Cargo.toml` declared `net_qa` and
//! `crates/gdtf_content_editor/src/app.rs` wired the listener behind
//! `cfg(all(debug_assertions, feature = "net_qa"))`. The editor's whole QA
//! listener therefore existed only inside `cargo dtest` / `cargo dclippy`, and
//! `NetQaEditorPlugin::from_env` — the constructor the binary reaches — had
//! never been executed by anything.
//!
//! The check is DERIVED, not a hand-kept list of the binaries that happen to
//! exist today: for every tracked `bins/*/Cargo.toml`, every inline
//! path-dependency it declares is read, and any dependency whose own manifest
//! declares a `net_qa` feature obliges the binary to declare
//! `net_qa = ["<dep>/net_qa"]`. A new binary that names its library the inline
//! `name = { path = "…" }` way — the style all three binaries use today — is
//! covered the day it lands; one written as a `[dependencies.foo]` sub-table
//! falls outside the reader (see `manifest.rs`) and needs a `REQUIRED_PAIRS`
//! entry to be checked. On top of that derived walk, `REQUIRED_PAIRS` in
//! `check.rs` names the two pairs that must have been reached and satisfied, so
//! a regression in the manifest reader cannot quietly make the guard vacuous.
//!
//! Reads manifests as text with a deliberately small TOML subset reader (see
//! `manifest.rs`) rather than taking a `toml` dependency for one guard — the
//! same std-only, zero-cargo-invocation shape as the sibling `module_layout`
//! and `docs_path_truth` suites, whose tree enumeration and single-final-assert
//! reporting this suite follows. Paths stay plain `String`s here for the same
//! reason they do in those two: they are file paths in a guard's own scratch
//! enumeration, not domain values the game models.
//!
//! The repo root defaults to `CARGO_MANIFEST_DIR/../..` and can be overridden
//! via the `GDTF_BIN_PASSTHROUGH_ROOT` env var (the sibling guards' recipe, for
//! pointing the built guard at another checkout).

mod check;
mod manifest;
mod tree;
