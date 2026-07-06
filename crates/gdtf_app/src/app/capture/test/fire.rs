//! The dev fire trigger: the `FireAtFrame` / `FireModeOverride` parse gates + the
//! headless drive of the REAL `FireRequested` path.

// The parse cores + the fire system are not re-exported from `mod.rs` (only
// `DevCapturePlugin` is, for the binary), so reach them through their home submodules.
use super::super::{
    trigger_config::{FireAtFrame, FireConfig, FireModeOverride},
    triggers::trigger_fire_at_frame,
};

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
    use gdtf_battle_sim::weapon::ModeKind;

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
        acts::FireRequested,
        battle::PlayerFaction,
        prelude::{Cell, CellLevel, Faction, Level, Position},
        weapon::{FireMode, FireModeSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent},
    };
    use gdtf_test_utils::{MessageProbePlugin, probed};

    use super::super::trigger_config::FireAtFrame;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<FireRequested>()
        // The generic GTW-576 probe — its `Last`-schedule drain captures the same
        // update's emission.
        .add_plugins(MessageProbePlugin::<FireRequested>::default());

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
    app.add_systems(Update, trigger_fire_at_frame);

    app.update();
    app.update();
    let fires = probed::<FireRequested>(&app);
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
        acts::FireRequested,
        battle::PlayerFaction,
        prelude::{Cell, CellLevel, Faction, Level, Position},
    };
    use gdtf_test_utils::{MessageProbePlugin, probed};

    use super::super::trigger_config::FireAtFrame;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<FireRequested>()
        // The generic GTW-576 probe — its `Last`-schedule drain captures the same
        // update's emission.
        .add_plugins(MessageProbePlugin::<FireRequested>::default());

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
    app.add_systems(Update, trigger_fire_at_frame);

    // Frames 1 + 2 must NOT fire.
    app.update();
    app.update();
    assert!(
        probed::<FireRequested>(&app).is_empty(),
        "the trigger must be silent before the target frame",
    );

    // Frame 3 fires exactly one FireRequested at the near enemy.
    app.update();
    let fires = probed::<FireRequested>(&app);
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
        probed::<FireRequested>(&app).len(),
        1,
        "the trigger fires exactly once",
    );
}
