//! The env-var RESOLUTION seam of the DEV-ONLY capture affordance (GTW-590): one raw
//! snapshot of every `GDTF_CAPTURE_*` / trigger env var ([`RawCaptureEnv`]), one PURE
//! resolution of that snapshot into the plugin's sub-configs ([`resolve`]), and one
//! LOUD diagnostic per silently-misconfigurable combination
//! ([`CaptureConfigWarning`]).
//!
//! Before GTW-590 each config type read its own env var and every bad value fell back
//! SILENTLY (an unparseable frame used the default, a blank path or a typo'd frames
//! list just switched the affordance off) — four QA runs produced zero PNGs with zero
//! evidence. This module is the fix's loudness half: `DevCapturePlugin::from_env`
//! snapshots the vars once, resolves them purely, and `warn!`s every diagnostic, so a
//! misconfiguration is one grep away. The tests drive [`resolve`] with injected
//! snapshots — the REAL production path minus the process-global env read.

use std::{env, fmt};

use super::capture_config::{
    CAPTURE_FRAME_ENV, CAPTURE_FRAMES_ENV, CAPTURE_PATH_ENV, CaptureConfig, CaptureFrame,
    CaptureFrames, parse_capture_path,
};
use crate::dev::drive::trigger_config::{
    FALL_AT_FRAME_ENV, FIRE_AT_FRAME_ENV, FIRE_MODE_ENV, FallAtFrame, FallConfig, FireAtFrame,
    FireConfig, FireModeOverride,
};

/// A raw, read-once snapshot of every env var the capture affordance consults.
///
/// Raw env-var VALUES are framework plumbing (the same reasoning as
/// [`CaptureConfig`]'s `PathBuf`), not domain values, so the no-bare-types rule does
/// not apply to the `Option<String>` fields. `pub(super)` fields: the sibling tests
/// build injected snapshots via struct literal.
#[derive(Debug, Clone, Default)]
pub(super) struct RawCaptureEnv {
    /// `GDTF_CAPTURE_PATH` — the output PNG base path (the capture opt-in gate).
    pub(super) path:      Option<String>,
    /// `GDTF_CAPTURE_FRAME` — the single-frame wait.
    pub(super) frame:     Option<String>,
    /// `GDTF_CAPTURE_FRAMES` — the multi-frame comma-list (wins over `frame`).
    pub(super) frames:    Option<String>,
    /// `GDTF_FIRE_AT_FRAME` — the scripted fire-trigger frame.
    pub(super) fire_at:   Option<String>,
    /// `GDTF_FIRE_MODE` — the fire-trigger mode override.
    pub(super) fire_mode: Option<String>,
    /// `GDTF_FALL_AT_FRAME` — the scripted fall-trigger frame.
    pub(super) fall_at:   Option<String>,
}

impl RawCaptureEnv {
    /// Snapshot the six env vars — the ONE process-global read of the affordance,
    /// taken at plugin construction (register time), never per frame.
    #[must_use]
    pub(super) fn from_env() -> Self {
        Self {
            path:      env::var(CAPTURE_PATH_ENV).ok(),
            frame:     env::var(CAPTURE_FRAME_ENV).ok(),
            frames:    env::var(CAPTURE_FRAMES_ENV).ok(),
            fire_at:   env::var(FIRE_AT_FRAME_ENV).ok(),
            fire_mode: env::var(FIRE_MODE_ENV).ok(),
            fall_at:   env::var(FALL_AT_FRAME_ENV).ok(),
        }
    }
}

/// One loud diagnostic for a set-but-ineffective capture env var (GTW-590 C3: zero
/// silent drops). Each variant renders (via [`fmt::Display`]) the exact `warn!` line
/// `DevCapturePlugin::from_env` emits, naming the variable, the offending value, and
/// the consequence — the tests pin both the classification and the rendered text.
/// Raw values ride as plain `String`s (rendered-message plumbing, the `RonSaveError`
/// precedent — not domain values).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum CaptureConfigWarning {
    /// `GDTF_CAPTURE_PATH` is set but blank / whitespace: capture stays OFF.
    BlankCapturePath,
    /// A capture frame var is set while `GDTF_CAPTURE_PATH` is not set at all: no
    /// frame will be captured. Carries the offending variable name.
    CaptureVarWithoutPath(&'static str),
    /// `GDTF_CAPTURE_FRAME` is set but not a valid `u32`: the default wait is used.
    InvalidCaptureFrame(String),
    /// One non-empty `GDTF_CAPTURE_FRAMES` entry is not a valid `u32`: that frame is
    /// dropped from the schedule.
    InvalidCaptureFramesEntry(String),
    /// `GDTF_CAPTURE_FRAMES` is set but holds NO valid entry: the single-frame
    /// schedule is used instead.
    CaptureFramesAllInvalid(String),
    /// `GDTF_FIRE_AT_FRAME` is set but not a valid `u32`: the fire trigger stays OFF.
    InvalidFireAtFrame(String),
    /// `GDTF_FIRE_MODE` is set but not a recognised mode name: the resident fire mode
    /// is used.
    InvalidFireMode(String),
    /// `GDTF_FIRE_MODE` parsed but `GDTF_FIRE_AT_FRAME` is absent / invalid: the
    /// override does nothing.
    FireModeWithoutTrigger,
    /// `GDTF_FALL_AT_FRAME` is set but not a valid `u32`: the fall trigger stays OFF.
    InvalidFallAtFrame(String),
}

impl fmt::Display for CaptureConfigWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BlankCapturePath => {
                write!(f, "{CAPTURE_PATH_ENV} is set but blank; capture stays OFF")
            }
            Self::CaptureVarWithoutPath(var) => write!(
                f,
                "{var} is set but {CAPTURE_PATH_ENV} is not; no frame will be captured"
            ),
            Self::InvalidCaptureFrame(raw) => write!(
                f,
                "{CAPTURE_FRAME_ENV}={raw:?} is not a valid frame number; using the default \
                 ({} BattleRunning frames)",
                *CaptureFrame::DEFAULT
            ),
            Self::InvalidCaptureFramesEntry(entry) => write!(
                f,
                "{CAPTURE_FRAMES_ENV} entry {entry:?} is not a valid frame number; that frame \
                 is dropped from the capture schedule"
            ),
            Self::CaptureFramesAllInvalid(raw) => write!(
                f,
                "{CAPTURE_FRAMES_ENV}={raw:?} holds no valid frame number; falling back to \
                 the single-frame schedule"
            ),
            Self::InvalidFireAtFrame(raw) => write!(
                f,
                "{FIRE_AT_FRAME_ENV}={raw:?} is not a valid frame number; the fire trigger \
                 stays OFF"
            ),
            Self::InvalidFireMode(raw) => write!(
                f,
                "{FIRE_MODE_ENV}={raw:?} is not a recognised mode (single|burst|full); using \
                 the resident fire mode"
            ),
            Self::FireModeWithoutTrigger => write!(
                f,
                "{FIRE_MODE_ENV} is set but {FIRE_AT_FRAME_ENV} is not; the mode override \
                 does nothing"
            ),
            Self::InvalidFallAtFrame(raw) => write!(
                f,
                "{FALL_AT_FRAME_ENV}={raw:?} is not a valid frame number; the fall trigger \
                 stays OFF"
            ),
        }
    }
}

/// The pure resolution of a [`RawCaptureEnv`] snapshot: the three optional
/// sub-configs plus every loud diagnostic.
#[derive(Debug, Clone)]
pub(super) struct ResolvedCaptureEnv {
    /// The capture sub-config (`None` = capture OFF).
    pub(super) capture:  Option<CaptureConfig>,
    /// The fire-trigger sub-config (`None` = trigger OFF).
    pub(super) fire:     Option<FireConfig>,
    /// The fall-trigger sub-config (`None` = trigger OFF).
    pub(super) fall:     Option<FallConfig>,
    /// Every set-but-ineffective diagnostic, in resolution order.
    pub(super) warnings: Vec<CaptureConfigWarning>,
}

/// Resolve a raw env snapshot into the plugin's sub-configs + loud diagnostics — the
/// PURE core `DevCapturePlugin::from_env` runs (and the tests drive with injected
/// snapshots). Delegates every value parse to the existing pure cores
/// ([`parse_capture_path`], [`CaptureFrame::parse`], [`CaptureFrames::parse`],
/// [`FireAtFrame::parse`], [`FireModeOverride::parse`], [`FallAtFrame::parse`]) so
/// resolution and parsing cannot drift; the classification around them decides which
/// silent fallback deserves a warning. Never panics.
pub(super) fn resolve(raw: &RawCaptureEnv) -> ResolvedCaptureEnv {
    let mut warnings = Vec::new();

    // Capture path: set-but-blank is a distinct, loud state (the var was typed, the
    // affordance still switched itself off).
    let path = parse_capture_path(raw.path.as_deref());
    if raw.path.is_some() && path.is_none() {
        warnings.push(CaptureConfigWarning::BlankCapturePath);
    }

    // Single-frame wait: an unparseable value silently used the default before GTW-590.
    // The `u32` probe mirrors `CaptureFrame::parse`'s internal accept test exactly.
    let fallback = CaptureFrame::parse(raw.frame.as_deref());
    if let Some(value) = raw.frame.as_deref()
        && value.trim().parse::<u32>().is_err()
    {
        warnings.push(CaptureConfigWarning::InvalidCaptureFrame(value.to_owned()));
    }

    // Multi-frame list: a typo'd entry silently DROPPED that frame before GTW-590 (the
    // QA-facing silent-drop class); an all-invalid list silently fell back.
    if let Some(list) = raw.frames.as_deref() {
        let mut any_valid = false;
        for entry in list.split(',').map(str::trim) {
            if entry.is_empty() {
                continue; // Bare formatting slack ("10,,60" / trailing comma), not a typo.
            }
            if entry.parse::<u32>().is_ok() {
                any_valid = true;
            } else {
                warnings.push(CaptureConfigWarning::InvalidCaptureFramesEntry(
                    entry.to_owned(),
                ));
            }
        }
        if !any_valid {
            warnings.push(CaptureConfigWarning::CaptureFramesAllInvalid(
                list.to_owned(),
            ));
        }
    }

    let capture = path.map(|path| CaptureConfig {
        path,
        frames: CaptureFrames::parse(raw.frames.as_deref(), fallback),
    });

    // Frame vars without ANY path var: the schedule was typed but nothing can write.
    // (The blank-path case is already covered by `BlankCapturePath` above.)
    if raw.path.is_none() {
        for (var, present) in [
            (CAPTURE_FRAME_ENV, raw.frame.is_some()),
            (CAPTURE_FRAMES_ENV, raw.frames.is_some()),
        ] {
            if present {
                warnings.push(CaptureConfigWarning::CaptureVarWithoutPath(var));
            }
        }
    }

    // Fire trigger: set-but-invalid left the trigger silently OFF before GTW-590.
    let fire_frame = FireAtFrame::parse(raw.fire_at.as_deref());
    if let Some(value) = raw.fire_at.as_deref()
        && fire_frame.is_none()
    {
        warnings.push(CaptureConfigWarning::InvalidFireAtFrame(value.to_owned()));
    }
    let mode = FireModeOverride::parse(raw.fire_mode.as_deref());
    if let Some(value) = raw.fire_mode.as_deref()
        && mode.is_none()
    {
        warnings.push(CaptureConfigWarning::InvalidFireMode(value.to_owned()));
    }
    if mode.is_some() && fire_frame.is_none() {
        warnings.push(CaptureConfigWarning::FireModeWithoutTrigger);
    }
    let fire = fire_frame.map(|frame| FireConfig { frame, mode });

    // Fall trigger: mirrors the fire trigger.
    let fall_frame = FallAtFrame::parse(raw.fall_at.as_deref());
    if let Some(value) = raw.fall_at.as_deref()
        && fall_frame.is_none()
    {
        warnings.push(CaptureConfigWarning::InvalidFallAtFrame(value.to_owned()));
    }
    let fall = fall_frame.map(|frame| FallConfig { frame });

    ResolvedCaptureEnv {
        capture,
        fire,
        fall,
        warnings,
    }
}
