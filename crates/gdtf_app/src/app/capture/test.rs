//! Unit tests for the DEV-ONLY screenshot / fire-trigger config + the headless
//! fire-trigger path (GTW-297, extended GTW-306).
//!
//! The config tests cover the pure parse cores: the [`capture_path`] gate, the
//! [`CaptureFrame`] / [`CaptureFrames`] / [`FireAtFrame`] parse, and the multi-frame
//! [`frame_path`] naming. They drive the REAL pure cores with INJECTED values rather than
//! mutating the process-global env vars (deterministic under parallel tests) — the same
//! code `from_env` / `capture_path` run.
//!
//! The ACTUAL screenshot capture needs a real render device, so it is NOT headless-
//! testable — it is verified by RUNNING the app. The fire trigger, by contrast, IS
//! headless: it only writes a `FireRequested` message, so the
//! [`trigger_fire_at_frame_emits_on_the_real_path`] test drives the REAL system on a
//! minimal app and asserts exactly one `FireRequested` at frame N.

use super::DevCapturePlugin;
// `CaptureFrame` etc. + the pure parse cores + the fire system are not re-exported from
// `mod.rs` (only `DevCapturePlugin` is, for the binary), so reach them through `plugin`.
use super::plugin::{
    CaptureFrame, CaptureFrames, FireAtFrame, FireConfig, FireModeOverride, capture_path,
    frame_path, parse_capture_path, trigger_fire_at_frame,
};

/// `CaptureFrame::DEFAULT` is 15 frames and `Default` agrees with it — the wait the
/// affordance uses when `GDTF_CAPTURE_FRAME` is unset, so the UI layout flushes before
/// the capture.
#[test]
fn capture_frame_default_is_fifteen() {
    assert_eq!(*CaptureFrame::DEFAULT, 15);
    assert_eq!(CaptureFrame::default(), CaptureFrame::DEFAULT);
}

/// The REAL frame parse ([`CaptureFrame::parse`], the core of `from_env`) takes a valid
/// `u32` and falls back to [`CaptureFrame::DEFAULT`] for an empty / non-numeric / absent
/// value — proving the parse path is wired without mutating the shared environment.
#[test]
fn capture_frame_parses_or_defaults() {
    assert_eq!(*CaptureFrame::parse(Some("0")), 0);
    assert_eq!(*CaptureFrame::parse(Some("7")), 7);
    assert_eq!(*CaptureFrame::parse(Some(" 42 ")), 42);
    for bad in ["", "  ", "x", "-1", "1.5", "12abc"] {
        assert_eq!(
            CaptureFrame::parse(Some(bad)),
            CaptureFrame::DEFAULT,
            "{bad:?} should fall back to the default frame",
        );
    }
    assert_eq!(CaptureFrame::parse(None), CaptureFrame::DEFAULT);
}

/// The REAL multi-frame parse ([`CaptureFrames::parse`], the core of `from_env`) turns a
/// `GDTF_CAPTURE_FRAMES` comma-list into a SORTED, DEDUPED, NON-EMPTY frame schedule —
/// and falls back to the single `GDTF_CAPTURE_FRAME` frame when the list is absent or has
/// no valid entry (the GTW-297 single-frame path stays working, GTW-306).
#[test]
fn capture_frames_parses_sorts_dedupes_or_falls_back() {
    let fallback = CaptureFrame::parse(Some("9"));

    // A real list: split, trim, sort, dedupe.
    let frames = CaptureFrames::parse(Some("16, 12 ,14,12,18"), fallback);
    let got: Vec<u32> = frames.iter().map(|f| **f).collect();
    assert_eq!(got, vec![12, 14, 16, 18], "sorted + deduped frame schedule");

    // A list with junk entries keeps only the valid u32s.
    let mixed = CaptureFrames::parse(Some("5,x,,7,-1"), fallback);
    let mixed_got: Vec<u32> = mixed.iter().map(|f| **f).collect();
    assert_eq!(mixed_got, vec![5, 7], "junk entries are dropped");

    // Absent / all-junk list -> the single fallback frame (single-frame path preserved).
    for empty in [None, Some(""), Some("   "), Some("x,,-1")] {
        let single = CaptureFrames::parse(empty, fallback);
        let single_got: Vec<u32> = single.iter().map(|f| **f).collect();
        assert_eq!(
            single_got,
            vec![9],
            "{empty:?} should fall back to the single GDTF_CAPTURE_FRAME frame",
        );
    }
}

/// The REAL multi-frame naming ([`frame_path`]) inserts a `.fNN` tag before the
/// extension so each frame writes its own PNG, and appends when there is no extension.
/// The single-frame path uses the exact base path (not this helper), so its filename is
/// unchanged — covered structurally by the schedule having one entry.
#[test]
fn frame_path_tags_the_frame_before_the_extension() {
    use std::path::{Path, PathBuf};

    assert_eq!(
        frame_path(Path::new("/abs/out.png"), CaptureFrame::parse(Some("12"))),
        PathBuf::from("/abs/out.f12.png"),
        "the frame tag goes before the extension",
    );
    assert_eq!(
        frame_path(Path::new("/abs/shot"), CaptureFrame::parse(Some("7"))),
        PathBuf::from("/abs/shot.f7"),
        "with no extension the tag is appended",
    );
}

/// The REAL fire-frame parse ([`FireAtFrame::parse`], the core of `from_env`) is `Some`
/// for a valid `u32` and `None` (trigger inert) for an absent / empty / non-numeric
/// value — the gate the dev fire-trigger keys on.
#[test]
fn fire_at_frame_parses_or_disables() {
    assert_eq!(FireAtFrame::parse(Some("12")), Some(FireAtFrame::new(12)));
    assert_eq!(FireAtFrame::parse(Some(" 3 ")), Some(FireAtFrame::new(3)));
    for off in [None, Some(""), Some("  "), Some("x"), Some("-1")] {
        assert!(
            FireAtFrame::parse(off).is_none(),
            "{off:?} should leave the fire trigger inert",
        );
    }
}

/// The REAL fire-mode parse ([`FireModeOverride::parse`], the core of `from_env`) maps a
/// recognised, case-insensitive mode name to its [`ModeKind`] and leaves the trigger on the
/// resident [`SelectedFireMode`] (`None`) for an absent / empty / unrecognised value — the
/// GTW-306 `GDTF_FIRE_MODE` gate that drives a multi-round capture volley.
#[test]
fn fire_mode_override_parses_or_disables() {
    use gdtf_battle_sim::ModeKind;

    assert_eq!(
        FireModeOverride::parse(Some("single")),
        Some(FireModeOverride::new(ModeKind::Single)),
    );
    assert_eq!(
        FireModeOverride::parse(Some(" Burst ")),
        Some(FireModeOverride::new(ModeKind::Burst)),
    );
    for full in ["full", "FULL", "full-auto", "fullauto"] {
        assert_eq!(
            FireModeOverride::parse(Some(full)),
            Some(FireModeOverride::new(ModeKind::Full)),
            "{full:?} should map to Full",
        );
    }
    for off in [None, Some(""), Some("  "), Some("rapid"), Some("x")] {
        assert!(
            FireModeOverride::parse(off).is_none(),
            "{off:?} should leave the trigger on SelectedFireMode",
        );
    }
}

/// GTW-306 — with a `GDTF_FIRE_MODE=full` override the dev fire-trigger fires in the
/// shooter's authored `Full` mode (read off its `FireMode` selector) rather than the
/// resident `SelectedFireMode` (`Single`), so the FX capture gets a MULTI-ROUND volley to
/// stagger. The emitted `FireRequested` carries the `Full`-mode spec (more than one shot).
#[test]
fn trigger_fires_in_the_overridden_full_mode() {
    use bevy::prelude::*;
    use gdtf_battle_input::{SelectedFireMode, SelectedShooter};
    use gdtf_battle_sim::{
        Cell, CellLevel, Faction, FireMode, FireModeSpec, Level, ModeConeMult, ModeKind, ModeShots,
        ModeTuPercent, PlayerFaction, Position, acts::FireRequested,
    };

    use super::plugin::FireAtFrame;

    /// Collected `FireRequested` messages, so the test asserts on the real emission.
    #[derive(Resource, Default)]
    struct FireProbe(Vec<FireRequested>);

    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<FireRequested>()
        .init_resource::<FireProbe>();

    let player = Faction::new(0);
    let enemy_faction = Faction::new(1);
    app.insert_resource(PlayerFaction::new(player));
    // The resident selection is the default SINGLE mode — the override must beat it.
    app.insert_resource(SelectedFireMode::default());

    // An armed shooter offering Single (1 shot) + Full (8 shots).
    let single = FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.3),
        ModeShots::new(1),
    );
    let full = FireModeSpec::new(
        ModeKind::Full,
        ModeConeMult::new(1.7),
        ModeTuPercent::new(0.6),
        ModeShots::new(8),
    );
    let shooter = app
        .world_mut()
        .spawn((
            player,
            Position::new(CellLevel::new(Cell::new(2, 2), Level::new(0))),
            FireMode::new(vec![single, full]),
        ))
        .id();
    app.world_mut().spawn((
        enemy_faction,
        Position::new(CellLevel::new(Cell::new(5, 2), Level::new(0))),
    ));
    app.insert_resource(SelectedShooter::new(shooter));

    // Fire on frame 2, overriding to Full.
    app.insert_resource(FireConfig::new(
        FireAtFrame::new(2),
        Some(FireModeOverride::new(ModeKind::Full)),
    ));
    app.add_systems(
        Update,
        (
            trigger_fire_at_frame,
            |mut reader: MessageReader<FireRequested>, mut probe: ResMut<FireProbe>| {
                for msg in reader.read() {
                    probe.0.push(msg.clone());
                }
            },
        )
            .chain(),
    );

    app.update();
    app.update();
    let fires = &app.world().resource::<FireProbe>().0;
    assert_eq!(
        fires.len(),
        1,
        "exactly one FireRequested at the target frame"
    );
    let shot = &fires[0];
    assert_eq!(
        shot.mode.kind,
        ModeKind::Full,
        "the override must fire in the shooter's authored Full mode, not the resident Single",
    );
    assert!(
        *shot.mode.shots > 1,
        "the Full mode fires more than one round (so the FX stagger has rounds to spread)",
    );
}

/// The REAL capture-path gate ([`parse_capture_path`], the core of `capture_path`) is
/// `None` for an unset / empty / whitespace value and `Some` for a real path — the gate
/// the affordance keys on. Driving the injected core avoids racing the process-global
/// env var across parallel tests.
#[test]
fn capture_path_gate_rejects_empty() {
    assert!(
        parse_capture_path(None).is_none(),
        "unset path leaves the affordance inert",
    );
    assert!(
        parse_capture_path(Some("")).is_none(),
        "empty path leaves it inert",
    );
    assert!(
        parse_capture_path(Some("   ")).is_none(),
        "whitespace path leaves it inert",
    );
    assert_eq!(
        parse_capture_path(Some("/abs/out.png")),
        Some(std::path::PathBuf::from("/abs/out.png")),
        "a real path enables the affordance",
    );
}

/// `DevCapturePlugin::from_env` defers to the env-var gates: capture is enabled exactly
/// when [`capture_path`] returns a path, and a [`CaptureFrames`] schedule is configured
/// iff enabled. Reading via the real `from_env` path keeps the assertion honest without
/// injecting or mutating state.
///
/// `capture_enabled` / `capture_frames` / `fire_frame` are `#[cfg(test)]` inherent
/// surface (absent from the binary build, keeping it `dead_code`-clean), so this test
/// reaches them directly.
#[test]
fn from_env_capture_enabled_matches_the_gate() {
    let plugin = DevCapturePlugin::from_env();
    assert_eq!(
        plugin.capture_enabled(),
        capture_path().is_some(),
        "from_env capture must activate exactly when GDTF_CAPTURE_PATH is set",
    );
    // When capture is enabled, a frame schedule is configured; when inert, none is.
    assert_eq!(plugin.capture_enabled(), plugin.capture_frames().is_some());
    // The fire trigger is independent — its `Option` mirrors the GDTF_FIRE_AT_FRAME gate.
    assert_eq!(plugin.fire_frame(), FireAtFrame::from_env());
}

/// GTW-306 — `trigger_fire_at_frame` fires the selected player ganger at the nearest
/// enemy on the REAL fire path: at the configured frame it writes exactly one
/// `FireRequested` (the same message a left-click over an enemy emits, which the sim's
/// `dispatch_fire` resolves), aimed at the enemy's cell, and is silent on every other
/// frame.
///
/// Drives the REAL `trigger_fire_at_frame` system on a minimal app (registered in
/// `Update` minus the unrelated `BattleScapeState` sub-state gate — the same "real system,
/// real schedule, minus unrelated state wiring" idiom the auto-battle A1 test uses). The
/// shooter + enemy are seeded as entities with `Position` / `Faction`; the selection /
/// fire-mode / player-faction resources are seeded directly. One `app.update()` per frame.
///
/// PIN: goes red if the trigger fires on the wrong frame, fails to read the selection /
/// nearest enemy, or stops emitting on the real `FireRequested` path.
#[test]
fn trigger_fire_at_frame_emits_on_the_real_path() {
    use bevy::prelude::*;
    use gdtf_battle_input::{SelectedFireMode, SelectedShooter};
    use gdtf_battle_sim::{
        Cell, CellLevel, Faction, Level, PlayerFaction, Position, acts::FireRequested,
    };

    use super::plugin::FireAtFrame;

    /// Collected `FireRequested` messages, so the test asserts on the real emission.
    #[derive(Resource, Default)]
    struct FireProbe(Vec<FireRequested>);

    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<FireRequested>()
        .init_resource::<FireProbe>();

    let player = Faction::new(0);
    let enemy_faction = Faction::new(1);
    app.insert_resource(PlayerFaction::new(player));
    app.insert_resource(SelectedFireMode::default());

    // Player shooter at (2, 2, 0); a near enemy at (5, 2, 0) and a far one at (40, 40, 0)
    // — the trigger must pick the NEAR enemy.
    let shooter = app
        .world_mut()
        .spawn((
            player,
            Position::new(CellLevel::new(Cell::new(2, 2), Level::new(0))),
        ))
        .id();
    app.world_mut().spawn((
        enemy_faction,
        Position::new(CellLevel::new(Cell::new(5, 2), Level::new(0))),
    ));
    app.world_mut().spawn((
        enemy_faction,
        Position::new(CellLevel::new(Cell::new(40, 40), Level::new(0))),
    ));
    app.insert_resource(SelectedShooter::new(shooter));

    // Fire on the 3rd BattleRunning frame (no mode override — uses SelectedFireMode).
    app.insert_resource(FireConfig::new(FireAtFrame::new(3), None));
    app.add_systems(
        Update,
        (
            trigger_fire_at_frame,
            |mut reader: MessageReader<FireRequested>, mut probe: ResMut<FireProbe>| {
                for msg in reader.read() {
                    probe.0.push(msg.clone());
                }
            },
        )
            .chain(),
    );

    // Frames 1 + 2 must NOT fire.
    app.update();
    app.update();
    assert!(
        app.world().resource::<FireProbe>().0.is_empty(),
        "the trigger must be silent before the target frame",
    );

    // Frame 3 fires exactly one FireRequested at the near enemy.
    app.update();
    let fires = &app.world().resource::<FireProbe>().0;
    assert_eq!(
        fires.len(),
        1,
        "exactly one FireRequested at the target frame"
    );
    let shot = &fires[0];
    assert_eq!(shot.shooter, shooter, "the shooter is the selected ganger");
    assert_eq!(
        shot.target_cell,
        Cell::new(5, 2),
        "the target is the NEAREST enemy cell, not the far one",
    );
    assert_eq!(
        shot.target_level,
        Level::new(0),
        "target level is the enemy's"
    );

    // Subsequent frames emit nothing more (one-shot).
    app.update();
    app.update();
    assert_eq!(
        app.world().resource::<FireProbe>().0.len(),
        1,
        "the trigger fires exactly once",
    );
}
