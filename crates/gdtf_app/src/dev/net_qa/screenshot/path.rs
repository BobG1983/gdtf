//! Producing the confined, per-capture-UNIQUE on-disk path for a screenshot (GTW-740).
//!
//! Two properties this module guarantees, each in its own function:
//!
//! - **Confinement.** A [`ShotName`] arrives off the network and is UNTRUSTED: it may carry
//!   `../`, an absolute prefix, or nested directories. [`confine_shot_path`] reduces it to a
//!   single safe file component joined under [`QaShotDir`] so a capture can NEVER write
//!   outside that one directory.
//! - **Uniqueness.** Every claimed capture gets its OWN path — [`next_capture_path`] folds a
//!   monotonic [`ShotSequence`] into the file name (`<stem>_<n>.png`), so a repeated name (or
//!   a leftover from a prior run) never shares a file the pump could mistake for THIS
//!   capture's output. This is the `gdtf_screenshot::timestamped_path` "successive presses do
//!   not clobber each other" discipline applied to the QA pump.

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use gdtf_qa_protocol::ids::ShotName;
use gdtf_screenshot::CapturePath;

crate::support_item! {
    /// The directory every QA screenshot is confined to, relative to the run's working
    /// directory (the workspace root for `cargo run`).
    ///
    /// Named-newtype [`Resource`] over [`PathBuf`] (no-bare-types) with a [`Default`] of
    /// `target/qa_screenshots` — `target/` is writable in dev and excluded from release
    /// artifacts (the sibling `gdtf_screenshot` keybind writes under `target/screenshots/`).
    /// Every wire-supplied name is confined STRICTLY inside this directory. The inner is
    /// PRIVATE. A test overrides the resource to a temp directory (through
    /// [`new`](Self::new)) so a test run never touches the source tree; the production plugin
    /// uses the [`Default`]. The visibility flips to `pub` under `test-support` (the
    /// `test_support` ledger re-exports it so the T7 integration test can inject the temp
    /// directory), `pub(crate)` otherwise.
    #[derive(Resource, Clone, Debug, Deref)]
    struct QaShotDir(PathBuf);
}

impl QaShotDir {
    crate::support_item! {
        /// Wrap an explicit confinement directory — a test's temp-dir override (the
        /// production plugin uses the [`Default`]).
        ///
        /// Gated to test / `test-support` builds: the production wiring only ever uses the
        /// [`Default`], so a release build never compiles this constructor.
        #[cfg(any(test, feature = "test-support"))]
        const fn new(dir: PathBuf) -> Self {
            Self(dir)
        }
    }
}

impl Default for QaShotDir {
    /// The wiring default: `target/qa_screenshots`.
    fn default() -> Self {
        Self(PathBuf::from("target/qa_screenshots"))
    }
}

/// A monotonic per-capture counter that makes every capture's output path unique.
///
/// Private-inner newtype [`Resource`] over `u64` (no-bare-types): the pump
/// [`advance`](Self::advance)s it once per claimed capture and folds the value into the file
/// name (`<stem>_<n>.png`). Uniqueness means a repeated name — or a decodable PNG left at a
/// path by an earlier capture — never becomes a file a later capture's poll could mistake for
/// its own output (the clobber hazard the `gdtf_screenshot` keybind solves with a unique
/// per-press name).
#[derive(Resource, Clone, Copy, Debug, Default, Deref)]
pub(in crate::dev::net_qa) struct ShotSequence(u64);

impl ShotSequence {
    /// Take the current value and advance the counter, so each claimed capture is handed a
    /// DISTINCT sequence number (and therefore a distinct output path).
    const fn advance(&mut self) -> Self {
        let current = *self;
        self.0 += 1;
        current
    }
}

/// The file stem used when a request supplies no name (or a name that sanitizes to
/// nothing). A fixed label handed to a path, not a domain value.
const DEFAULT_STEM: &str = "qa_shot";

/// Confine an untrusted wire [`ShotName`] to a `.png` path strictly inside `dir`.
///
/// A client name may carry `../`, an absolute prefix, or nested directories.
/// [`Path::file_name`] discards every leading directory / traversal segment, the remainder
/// is filtered down to `[A-Za-z0-9_-]`, and that SINGLE component is joined under `dir` — so
/// the result can never escape it. An empty / fully-stripped name falls back to
/// [`DEFAULT_STEM`]; the lone `.png` extension is applied here (any client-supplied
/// extension is dropped by the filter). This proves CONFINEMENT only; [`next_capture_path`]
/// adds the per-capture uniqueness suffix on top.
pub(in crate::dev::net_qa) fn confine_shot_path(
    dir: &QaShotDir,
    name: Option<&ShotName>,
) -> CapturePath {
    let raw = name.map_or("", |shot| shot.as_str());
    // Keep only the final path component (drops any directory / `..` segment) ...
    let component = Path::new(raw)
        .file_name()
        .and_then(|last| last.to_str())
        .unwrap_or("");
    // ... drop a client-supplied `.png` so it is not mangled into the stem (re-applied
    // below), then filter to a safe single-component charset.
    let stem_src = component.strip_suffix(".png").unwrap_or(component);
    let cleaned: String = stem_src
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        .collect();
    let stem = if cleaned.is_empty() {
        DEFAULT_STEM
    } else {
        cleaned.as_str()
    };
    CapturePath::new(dir.join(format!("{stem}.png")))
}

/// Fold a [`ShotSequence`] value into an already-confined path's file name, yielding
/// `<stem>_<n>.png` in the SAME directory.
///
/// `base` is the confined single-component path [`confine_shot_path`] produced, so replacing
/// only its file name (keeping its parent, appending the numeric suffix + `.png`) stays
/// strictly under [`QaShotDir`] — the confinement guarantee is preserved.
fn sequenced_path(base: &CapturePath, seq: ShotSequence) -> CapturePath {
    let stem = base
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or(DEFAULT_STEM);
    let file = format!("{stem}_{}.png", *seq);
    let path = base
        .parent()
        .map_or_else(|| PathBuf::from(&file), |dir| dir.join(&file));
    CapturePath::new(path)
}

/// The pump's path entry point: confine the untrusted `name` under `dir`, then make it
/// UNIQUE by folding in the next [`ShotSequence`] value — so no two claimed captures ever
/// share a file to clobber (a repeated name, or a leftover from a prior run).
///
/// Advancing `seq` is the whole point: each call returns a path with a distinct sequence
/// number, and the confinement of [`confine_shot_path`] carries through unchanged.
pub(in crate::dev::net_qa) fn next_capture_path(
    dir: &QaShotDir,
    name: Option<&ShotName>,
    seq: &mut ShotSequence,
) -> CapturePath {
    let base = confine_shot_path(dir, name);
    sequenced_path(&base, seq.advance())
}
