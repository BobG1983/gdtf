//! GTW-590 pins on the env-var RESOLUTION seam: the pinned QA snapshot resolves to
//! the exact configured frame set (C4a), `from_env` defers to the same pure core, and
//! every silently-misconfigurable env combination raises its LOUD diagnostic with the
//! exact rendered `warn!` text (C4c). All driven through the REAL [`resolve`] path
//! with INJECTED snapshots — no process-global env mutation (deterministic under
//! parallel tests).

use super::super::{
    DevCapturePlugin,
    resolve::{CaptureConfigWarning, RawCaptureEnv, resolve},
};

/// Shorthand: a snapshot with only the given fields set.
fn snapshot(
    path: Option<&str>,
    frames: Option<&str>,
    fire_at: Option<&str>,
    fire_mode: Option<&str>,
) -> RawCaptureEnv {
    RawCaptureEnv {
        path: path.map(str::to_owned),
        frames: frames.map(str::to_owned),
        fire_at: fire_at.map(str::to_owned),
        fire_mode: fire_mode.map(str::to_owned),
        ..RawCaptureEnv::default()
    }
}

/// C4a — the PINNED GTW-590 QA invocation's env set (`GDTF_CAPTURE_PATH` +
/// `GDTF_CAPTURE_FRAMES="10,60,150,300"`) resolves to a capture config with exactly
/// that frame schedule, no triggers, and ZERO warnings — through the REAL
/// [`resolve`] → [`DevCapturePlugin::from_resolved`] construction path the production
/// `from_env` runs.
#[test]
fn pinned_qa_snapshot_resolves_to_the_exact_frame_schedule() {
    let raw = snapshot(
        Some("/tmp/gtw627_shots/frame.png"),
        Some("10,60,150,300"),
        None,
        None,
    );
    let resolved = resolve(&raw);
    assert!(
        resolved.warnings.is_empty(),
        "a clean QA snapshot raises no diagnostics: {:?}",
        resolved.warnings,
    );
    let plugin = DevCapturePlugin::from_resolved(resolved);
    assert!(plugin.capture_enabled(), "the capture sub-affordance is ON");
    let frames: Vec<u32> = plugin
        .capture_frames()
        .map(|frames| frames.iter().map(|frame| **frame).collect())
        .unwrap_or_default();
    assert_eq!(
        frames,
        vec![10, 60, 150, 300],
        "every configured frame is scheduled",
    );
    assert_eq!(plugin.fire_frame(), None, "no fire trigger configured");
    assert_eq!(plugin.fall_frame(), None, "no fall trigger configured");
}

/// C4c — every set-but-ineffective env combination raises its diagnostic. Each case
/// pins the CLASSIFICATION (which warning, in resolution order) for the exact silent
/// failure it retires; the rendered text is pinned separately below.
#[test]
fn silent_misconfigurations_raise_their_diagnostics() {
    use CaptureConfigWarning as W;

    // Blank path: the var was typed, capture still switched itself off.
    let blank = resolve(&snapshot(Some("   "), None, None, None));
    assert!(blank.capture.is_none(), "blank path leaves capture OFF");
    assert_eq!(blank.warnings, vec![W::BlankCapturePath]);

    // Frame vars without ANY path var: a schedule that can never write.
    let no_path = resolve(&snapshot(None, Some("10,60"), None, None));
    assert!(no_path.capture.is_none());
    assert_eq!(
        no_path.warnings,
        vec![W::CaptureVarWithoutPath("GDTF_CAPTURE_FRAMES")],
    );

    // An unparseable single-frame wait: the default is used, loudly.
    let bad_frame = resolve(&RawCaptureEnv {
        path: Some("/tmp/out.png".to_owned()),
        frame: Some("soon".to_owned()),
        ..RawCaptureEnv::default()
    });
    assert_eq!(
        bad_frame.warnings,
        vec![W::InvalidCaptureFrame("soon".to_owned())],
    );

    // A typo'd frames entry: THAT frame silently vanished from the schedule before
    // GTW-590 (the QA-facing silent-drop class).
    let typo = resolve(&snapshot(
        Some("/tmp/out.png"),
        Some("10,6O,300"),
        None,
        None,
    ));
    assert_eq!(
        typo.warnings,
        vec![W::InvalidCaptureFramesEntry("6O".to_owned())],
    );
    let frames: Vec<u32> = typo
        .capture
        .as_ref()
        .map(|config| config.frames.iter().map(|frame| **frame).collect())
        .unwrap_or_default();
    assert_eq!(frames, vec![10, 300], "the valid frames still capture");

    // An all-invalid list: single-frame fallback, loudly (per-entry + summary).
    let all_bad = resolve(&snapshot(Some("/tmp/out.png"), Some("x,y"), None, None));
    assert_eq!(
        all_bad.warnings,
        vec![
            W::InvalidCaptureFramesEntry("x".to_owned()),
            W::InvalidCaptureFramesEntry("y".to_owned()),
            W::CaptureFramesAllInvalid("x,y".to_owned()),
        ],
    );

    // An unparseable fire frame: the trigger stayed silently OFF before GTW-590.
    let bad_fire = resolve(&snapshot(None, None, Some("x"), None));
    assert!(bad_fire.fire.is_none());
    assert_eq!(
        bad_fire.warnings,
        vec![W::InvalidFireAtFrame("x".to_owned())],
    );

    // An unrecognised fire mode: the resident mode was silently used.
    let bad_mode = resolve(&snapshot(None, None, Some("8"), Some("sideways")));
    assert!(bad_mode.fire.is_some(), "the trigger itself still fires");
    assert_eq!(
        bad_mode.warnings,
        vec![W::InvalidFireMode("sideways".to_owned())],
    );

    // A mode override without its trigger: the override silently did nothing.
    let orphan_mode = resolve(&snapshot(None, None, None, Some("full")));
    assert_eq!(orphan_mode.warnings, vec![W::FireModeWithoutTrigger]);

    // An unparseable fall frame: the trigger stayed silently OFF (GTW-529 mirror).
    let bad_fall = resolve(&RawCaptureEnv {
        fall_at: Some("-2".to_owned()),
        ..RawCaptureEnv::default()
    });
    assert!(bad_fall.fall.is_none());
    assert_eq!(
        bad_fall.warnings,
        vec![W::InvalidFallAtFrame("-2".to_owned())],
    );
}

/// C4c — the diagnostics RENDER the exact loud lines `from_env` `warn!`s (each is
/// prefixed `dev-capture: ` at the emit site), naming the variable, the offending
/// value, and the consequence. QA greps for these; a reworded silent variant is a
/// regression.
#[test]
fn diagnostics_render_the_loud_lines() {
    use CaptureConfigWarning as W;

    let cases = [
        (
            W::BlankCapturePath,
            "GDTF_CAPTURE_PATH is set but blank; capture stays OFF".to_owned(),
        ),
        (
            W::CaptureVarWithoutPath("GDTF_CAPTURE_FRAMES"),
            "GDTF_CAPTURE_FRAMES is set but GDTF_CAPTURE_PATH is not; no frame will be \
             captured"
                .to_owned(),
        ),
        (
            W::InvalidCaptureFrame("soon".to_owned()),
            "GDTF_CAPTURE_FRAME=\"soon\" is not a valid frame number; using the default \
             (15 BattleRunning frames)"
                .to_owned(),
        ),
        (
            W::InvalidCaptureFramesEntry("6O".to_owned()),
            "GDTF_CAPTURE_FRAMES entry \"6O\" is not a valid frame number; that frame is \
             dropped from the capture schedule"
                .to_owned(),
        ),
        (
            W::CaptureFramesAllInvalid("x,y".to_owned()),
            "GDTF_CAPTURE_FRAMES=\"x,y\" holds no valid frame number; falling back to the \
             single-frame schedule"
                .to_owned(),
        ),
        (
            W::InvalidFireAtFrame("x".to_owned()),
            "GDTF_FIRE_AT_FRAME=\"x\" is not a valid frame number; the fire trigger stays \
             OFF"
            .to_owned(),
        ),
        (
            W::InvalidFireMode("sideways".to_owned()),
            "GDTF_FIRE_MODE=\"sideways\" is not a recognised mode (single|burst|full); \
             using the resident fire mode"
                .to_owned(),
        ),
        (
            W::FireModeWithoutTrigger,
            "GDTF_FIRE_MODE is set but GDTF_FIRE_AT_FRAME is not; the mode override does \
             nothing"
                .to_owned(),
        ),
        (
            W::InvalidFallAtFrame("-2".to_owned()),
            "GDTF_FALL_AT_FRAME=\"-2\" is not a valid frame number; the fall trigger stays \
             OFF"
            .to_owned(),
        ),
    ];
    for (warning, expected) in cases {
        assert_eq!(warning.to_string(), expected, "loud line for {warning:?}");
    }
}

/// `from_env` defers to the SAME pure core the tests drive: constructing from the
/// process env equals constructing from [`resolve`] of the live [`RawCaptureEnv`]
/// snapshot (read-only — nothing here mutates the environment).
#[test]
fn from_env_defers_to_the_resolved_snapshot() {
    let via_env = DevCapturePlugin::from_env();
    let via_core = DevCapturePlugin::from_resolved(resolve(&RawCaptureEnv::from_env()));
    assert_eq!(via_env.capture_enabled(), via_core.capture_enabled());
    assert_eq!(via_env.capture_frames(), via_core.capture_frames());
    assert_eq!(via_env.fire_frame(), via_core.fire_frame());
    assert_eq!(via_env.fall_frame(), via_core.fall_frame());
}
