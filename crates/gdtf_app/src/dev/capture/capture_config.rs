//! The capture-output env config half of the DEV-ONLY capture affordance: which
//! `BattleRunning` frames to capture (`GDTF_CAPTURE_FRAME` / `GDTF_CAPTURE_FRAMES`)
//! and where the PNGs land (`GDTF_CAPTURE_PATH`), plus the multi-frame path naming.
//! Split out of the sibling `plugin` module (GTW-583); see its header for the full
//! affordance rationale.

use std::path::{Path, PathBuf};

use bevy::prelude::*;

/// The `GDTF_CAPTURE_PATH` environment variable: the absolute path of the output
/// PNG. Setting it (in a `dev_capture` debug build) opts into the capture affordance.
/// Read (once) by the sibling `resolve` module's env snapshot.
pub(super) const CAPTURE_PATH_ENV: &str = "GDTF_CAPTURE_PATH";

/// The `GDTF_CAPTURE_FRAME` environment variable: how many `BattleRunning` frames to
/// wait before capturing a SINGLE frame (parsed into a [`CaptureFrame`]).
pub(super) const CAPTURE_FRAME_ENV: &str = "GDTF_CAPTURE_FRAME";

/// The `GDTF_CAPTURE_FRAMES` environment variable: a comma-separated list of
/// `BattleRunning` frames to capture in ONE run (parsed into a [`CaptureFrames`]). When
/// set it wins over [`CAPTURE_FRAME_ENV`]; one PNG is written per listed frame.
pub(super) const CAPTURE_FRAMES_ENV: &str = "GDTF_CAPTURE_FRAMES";

/// How many frames AFTER the battle is running to wait before capturing a single frame.
///
/// The capture system counts only frames spent in
/// [`BattleScapeState::BattleRunning`](crate::states::BattleScapeState::BattleRunning); once its `Local` counter reaches this value it
/// fires the screenshot. Waiting a handful of frames lets Bevy's UI layout flush so the
/// captured HUD is settled rather than mid-layout.
///
/// A named newtype over `u32` (no-bare-types). `pub(crate)`: referenced only by the
/// in-crate wiring + config tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deref)]
pub(crate) struct CaptureFrame(u32);

impl CaptureFrame {
    /// The default wait: 15 `BattleRunning` frames, enough for the UI layout to flush.
    pub(crate) const DEFAULT: Self = Self(15);

    /// Build a capture frame from its raw frame index. `pub(super)`: the sibling
    /// `screenshot` module's frame counter constructs the current frame through this
    /// (the tuple constructor cannot cross the module boundary onto the private inner).
    #[must_use]
    pub(super) const fn new(frame: u32) -> Self {
        Self(frame)
    }

    /// Parse a raw env-var value into a [`CaptureFrame`], falling back to
    /// [`CaptureFrame::DEFAULT`] when the value is absent, empty / whitespace, or not a
    /// valid `u32`. A set-but-invalid value additionally raises a loud
    /// [`CaptureConfigWarning`](super::resolve::CaptureConfigWarning) in the sibling
    /// `resolve` module (GTW-590 — no silent fallbacks). Pure, so the config tests
    /// drive the REAL parse path with injected values (no env mutation).
    #[must_use]
    pub(crate) fn parse(value: Option<&str>) -> Self {
        value
            .and_then(|raw| raw.trim().parse::<u32>().ok())
            .map_or(Self::DEFAULT, Self)
    }
}

impl Default for CaptureFrame {
    /// The wiring default: [`CaptureFrame::DEFAULT`].
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The ordered, de-duplicated set of `BattleRunning` frames to capture in one run.
///
/// A named newtype over `Vec<CaptureFrame>` (no-bare-types: the capture schedule is a
/// domain value) holding a SORTED, DEDUPED, NON-EMPTY list. The single-frame
/// `GDTF_CAPTURE_FRAME` path is just the one-element case, so the capture system handles
/// both uniformly. `pub(crate)`: referenced only by the in-crate wiring + config tests.
#[derive(Debug, Clone, PartialEq, Eq, Deref)]
pub(crate) struct CaptureFrames(Vec<CaptureFrame>);

impl CaptureFrames {
    /// Parse a raw `GDTF_CAPTURE_FRAMES` comma-list into a [`CaptureFrames`], falling
    /// back to the single `fallback` frame when the list is absent, empty, or holds no
    /// valid `u32` entry. Splits on `,`, trims each entry, keeps the valid `u32`s, then
    /// SORTS + DEDUPES so the capture system fires each frame once in order. Pure —
    /// the env read + the loud set-but-invalid diagnostics live in the sibling
    /// `resolve` module (GTW-590); never panics.
    #[must_use]
    pub(crate) fn parse(list: Option<&str>, fallback: CaptureFrame) -> Self {
        let mut frames: Vec<CaptureFrame> = list
            .into_iter()
            .flat_map(|raw| raw.split(','))
            .filter_map(|entry| entry.trim().parse::<u32>().ok().map(CaptureFrame))
            .collect();
        frames.sort_unstable();
        frames.dedup();
        if frames.is_empty() {
            // No valid list entries -> the single-frame schedule (the `GDTF_CAPTURE_FRAME`
            // / default path stays working).
            frames.push(fallback);
        }
        Self(frames)
    }

    /// The last (highest) frame in the schedule — the LAST target frame, after which
    /// the capture observer sets `RunningState::Quit` to ride the shared shutdown
    /// cascade. Infallible: the list is non-empty by construction.
    #[must_use]
    pub(super) fn last_frame(&self) -> CaptureFrame {
        // `copied().max()` over a non-empty sorted list; the `unwrap_or` is a structural
        // safety net (never taken) that keeps the no-`unwrap` rule satisfied.
        self.0
            .iter()
            .copied()
            .max()
            .unwrap_or(CaptureFrame::DEFAULT)
    }

    /// Whether this schedule is the single-frame case (exactly one target frame). In
    /// that case the capture writes the exact `GDTF_CAPTURE_PATH` (no per-frame suffix),
    /// preserving the GTW-297 single-frame behavior.
    #[must_use]
    pub(super) const fn is_single(&self) -> bool {
        self.0.len() == 1
    }
}

impl std::fmt::Display for CaptureFrames {
    /// The human/QA-facing schedule rendering — the frame numbers as a comma list
    /// (`10,60,150,300`), used by the GTW-590 loud activation log.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for frame in &self.0 {
            if !first {
                write!(f, ",")?;
            }
            write!(f, "{}", **frame)?;
            first = false;
        }
        Ok(())
    }
}

/// The resolved capture configuration: where to write and which frames to capture.
///
/// Built by the sibling `resolve` module's env resolution, held by
/// [`DevCapturePlugin`](super::plugin::DevCapturePlugin) when the capture sub-affordance
/// is enabled, and inserted as a [`Resource`] so
/// [`capture_when_ready`](super::screenshot::capture_when_ready) can read both fields.
/// Framework-plumbing config (a path + a [`CaptureFrames`] newtype), not a domain value,
/// so the no-bare-types rule applies only to its `frames` field (which is the newtype).
/// `pub(crate)`: a purely internal resource, never re-exported.
#[derive(Resource, Debug, Clone)]
pub(crate) struct CaptureConfig {
    /// Absolute base path of the output PNG(s), handed to
    /// [`save_to_disk`](bevy::render::view::window::screenshot::save_to_disk). On the
    /// multi-frame path each frame is suffixed via [`frame_path`].
    pub(super) path:   PathBuf,
    /// Which `BattleRunning` frames to capture (one PNG per frame).
    pub(super) frames: CaptureFrames,
}

impl CaptureConfig {
    /// Build a single-frame capture config for `path` with the default schedule.
    /// Test-only inherent surface (the production path constructs it via struct
    /// literal in the `resolve` module; the GTW-590 output-dir pins drive
    /// [`ensure_output_dir`](super::diagnostics::ensure_output_dir) through this).
    /// `#[cfg(test)]` so the binary stays `dead_code`-clean.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn for_test(path: PathBuf) -> Self {
        Self {
            path,
            frames: CaptureFrames::parse(None, CaptureFrame::DEFAULT),
        }
    }
}

/// Apply the capture-path gate to a raw env-var value: `Some(path)` when it is set to a
/// non-empty (trimmed) value, `None` (affordance inert) when absent, empty, or all
/// whitespace. The pure core of the `resolve` module's [`CAPTURE_PATH_ENV`] resolution
/// (`GDTF_CAPTURE_PATH`), factored out so the config tests drive the REAL gate with
/// injected values (no env mutation). The `cfg!(all(debug_assertions, feature =
/// "dev_capture"))` half of the gate lives at the [`GdtfApp`](crate::GdtfApp) wiring
/// site, so a release / default build never even compiles the affordance in.
/// `pub(crate)`.
///
/// The path is framework plumbing handed straight to
/// [`save_to_disk`](bevy::render::view::window::screenshot::save_to_disk) — not a domain
/// value — so the no-bare-types rule does not apply to it.
///
/// GTW-510: delegates the trim/empty gate to the shared
/// [`gdtf_screenshot::parse_shot_path`] primitive (so the editor and the game share ONE
/// path-parse), then unwraps the returned [`CapturePath`](gdtf_screenshot::CapturePath)
/// back into the [`PathBuf`] the game's multi-frame [`CaptureConfig`] threads through.
#[must_use]
pub(crate) fn parse_capture_path(value: Option<&str>) -> Option<PathBuf> {
    gdtf_screenshot::parse_shot_path(value).map(|path| (*path).clone())
}

/// Insert a `.fNN` frame tag before a capture path's extension, e.g. `out.png` at frame
/// `12` -> `out.f12.png` (multi-frame). With no extension the tag is appended:
/// `shot` -> `shot.f12`. Used only on the multi-frame path; the single-frame path writes
/// the exact base path.
///
/// Pure path plumbing (no `World`) so the multi-frame naming is unit-testable.
/// `pub(crate)`.
#[must_use]
pub(crate) fn frame_path(base: &Path, frame: CaptureFrame) -> PathBuf {
    let mut tagged = base.to_path_buf();
    let stem = base
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("capture");
    let tagged_name = match base.extension().and_then(|e| e.to_str()) {
        Some(ext) => format!("{stem}.f{}.{ext}", *frame),
        None => format!("{stem}.f{}", *frame),
    };
    tagged.set_file_name(tagged_name);
    tagged
}
