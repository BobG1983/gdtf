//! The [`CapturePath`] output-path newtype and the pure env-value / stable-path helpers.
//!
//! A captured screenshot must land at a STABLE, agent-readable path so a QA pass can `Read` the PNG
//! and assert on the ACTUAL rendered layout (GTW-510). This module owns the ONE place a raw path
//! string becomes a typed [`CapturePath`]: [`parse_shot_path`] is the pure env-value gate every
//! consumer delegates to (so no scene re-implements the trim/empty rules), and [`timestamped_path`]
//! computes the keybind trigger's `target/screenshots/<name>-<secs>.png` default.

use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use bevy::prelude::*;

/// The resolved absolute path a captured screenshot PNG is written to.
///
/// A named newtype over [`PathBuf`] (no-bare-types): the capture output location is a domain value
/// the crate threads through as a [`Resource`], not a bare path. The inner is PRIVATE — build one
/// through [`CapturePath::new`] / [`parse_shot_path`], read it through the derived [`Deref`] (so
/// `.display()` / `.exists()` work) — so the constructed-only invariant (a trimmed, non-empty path)
/// cannot be sidestepped.
#[derive(Resource, Clone, Debug, PartialEq, Eq, Deref)]
pub struct CapturePath(PathBuf);

impl CapturePath {
    /// Wrap a resolved output path. Used by [`parse_shot_path`] and by a programmatic driver that
    /// already holds the exact [`PathBuf`] to capture to (a QA harness inserting the resource).
    #[must_use]
    pub const fn new(path: PathBuf) -> Self {
        Self(path)
    }
}

/// Apply the shot-path gate to a raw env-var value: `Some(path)` when the value is present and
/// non-empty after trimming, `None` (the affordance stays inert) when absent, empty, or all
/// whitespace.
///
/// This is the SINGLE pure path-parse the crate exposes — pure (no `World`, no env read) so every
/// consuming scene's capture hook drives the SAME emptiness/trim logic without duplicating it and
/// without a test having to mutate the process-global env var. The caller reads the env var
/// (`std::env::var(VAR).ok().as_deref()`) and hands the value here.
#[must_use]
pub fn parse_shot_path(value: Option<&str>) -> Option<CapturePath> {
    value
        .map(|raw| raw.trim().to_owned())
        .filter(|trimmed| !trimmed.is_empty())
        .map(|trimmed| CapturePath::new(PathBuf::from(trimmed)))
}

/// The stable dev-capture output directory, relative to the workspace root: `target/screenshots/`.
///
/// `target/` is writable in dev and excluded from release artifacts, so a keybind-triggered capture
/// lands in a known, agent-readable place without an env var. Framework plumbing (a directory
/// segment handed to [`PathBuf`]), not a domain value.
const SCREENSHOTS_DIR: &str = "target/screenshots";

/// Compute a stable, unique keybind-capture path under the `target/screenshots/` dir:
/// `target/screenshots/<name>-<unix-secs>.png`.
///
/// Used by the [`KeyboardCapturePlugin`](crate::KeyboardCapturePlugin) so an interactive dev press
/// writes to a KNOWN directory (agent-readable) with a per-press-unique filename (so successive
/// presses do not clobber each other). The `<name>` prefix lets a caller tag the scene
/// (`"editor"` / `"game"`). Uses [`SystemTime`] elapsed seconds for uniqueness; on the (impossible
/// in practice) clock-before-epoch case it falls back to `0` rather than panicking, so the path is
/// always well-formed.
#[must_use]
pub fn timestamped_path(name: &str) -> CapturePath {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    CapturePath::new(PathBuf::from(format!(
        "{SCREENSHOTS_DIR}/{name}-{secs}.png"
    )))
}

#[cfg(test)]
mod tests {
    use super::{SCREENSHOTS_DIR, parse_shot_path, timestamped_path};

    #[test]
    fn path_gate_accepts_non_empty_and_rejects_blank() {
        assert!(parse_shot_path(Some("/abs/out.png")).is_some());
        assert!(parse_shot_path(Some("  /trim/out.png  ")).is_some());
        assert!(parse_shot_path(Some("")).is_none());
        assert!(parse_shot_path(Some("   ")).is_none());
        assert!(parse_shot_path(None).is_none());
    }

    #[test]
    fn path_gate_trims_surrounding_whitespace() {
        let parsed = parse_shot_path(Some("  /trim/out.png  "));
        assert_eq!(
            parsed.map(|path| path.to_string_lossy().into_owned()),
            Some("/trim/out.png".to_owned())
        );
    }

    #[test]
    fn timestamped_path_is_under_screenshots_dir_and_png() {
        let path = timestamped_path("editor");
        let text = path.to_string_lossy();
        assert!(
            text.starts_with(SCREENSHOTS_DIR),
            "under target/screenshots: {text}"
        );
        assert!(text.contains("editor-"), "carries the scene tag: {text}");
        assert!(text.ends_with(".png"), "is a PNG: {text}");
    }
}
