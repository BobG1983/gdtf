//! GTW-626 docs path-truth gate — every repo path referenced from `docs/` and
//! `.claude/rules/` must resolve against the live tree, so a module move can
//! never silently strand design canon again (the GTW-445 lesson recurring: the
//! GTW-385 regroup stranded `docs/combat/` + `docs/decisions/` source
//! attributions, and a binding rule kept citing the deleted
//! `docs/architecture.md`).
//!
//! The suite scans every tracked `.md`/`.txt` file under the two roots (via
//! `git ls-files`, with a std fs-walk fallback) and extracts two reference
//! shapes:
//!
//! - **root-anchored path runs** — `docs/`-, `crates/`-, `bins/`-, `assets/`-,
//!   `content/`-, `.claude/`- or `.cargo/`-prefixed paths anywhere in the text.
//!   A `content/`-prefixed run may also resolve under `assets/` (the Bevy
//!   asset root), because docs legitimately quote asset-relative paths such as
//!   `load_folder("content/terrain")` and hot-reload log lines;
//! - **inline markdown link targets** — `[text](target)`, resolved relative to
//!   the containing file (repo-relative when the target starts with `/`).
//!
//! Runs continuing into a glob or placeholder (`*`, `<`) are not literal path
//! claims and are skipped, as are the pinned historical/transient pairs in
//! `check.rs`. Std-only, zero cargo invocations; the repo root defaults to
//! `CARGO_MANIFEST_DIR/../..` and can be overridden via the
//! `GDTF_DOCS_PATH_ROOT` env var (for pointing the built guard at another
//! checkout).

mod check;
mod refs;
mod tree;
