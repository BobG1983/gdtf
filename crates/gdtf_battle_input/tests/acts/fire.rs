//! Fire-mode default on select + left-click fire emission + `can_fire`
//! fail-closed (AC1/AC3/AC4).

use bevy::prelude::*;
use gdtf_battle_input::{InspectTarget, SelectedFireMode};
use gdtf_battle_sim::{
    ganger::{Aiming, TuMax},
    magazine::{LoadedRounds, Magazine, ReloadTu},
    prelude::{CellLevel, Direction, Level, LifeState, OccupancyGrid, StanceKind, Tu},
    tuning::CombatTuning,
    visibility::SquadVisibility,
    weapon::{FireModeSpec, MagazineSize},
};
use gdtf_test_utils::press_left;

use super::harness::*;

/// Empties the wielded WEAPON entity's magazine (GTW-323 slice 3: the fire guard reads
/// `ganger → Wields → weapon → Magazine`, so an empty-magazine test must empty the WEAPON,
/// not the ganger). No-op if the ganger wields no weapon.
fn empty_wielded_magazine(app: &mut App, ganger: Entity) {
    if let Some(weapon) = app
        .world()
        .get::<gdtf_battle_sim::weapon::Wields>(ganger)
        .and_then(gdtf_battle_sim::weapon::Wields::weapon)
    {
        app.world_mut().entity_mut(weapon).insert(Magazine::new(
            LoadedRounds::new(0),
            MagazineSize::new(30),
            ReloadTu::new(12),
        ));
    }
}

/// Places an ENEMY-faction occupant in the occupancy grid at `cell` (so the FIRE branch
/// of the unified left-click decision sees a non-player target there), returning its
/// entity. The enemy carries only a `Faction` — the fire path reads the SHOOTER's firing
/// components, never the target's.
///
/// GTW-11 — it ALSO marks `cell` squad-VISIBLE in the fog, the realistic battle state (you fire
/// on a SEEN enemy): without this the new targeting-fog rung in `decide_left_click` would refuse
/// every fire (fail-closed on a cell absent from the default-empty `SquadVisibility`). The
/// can_fire-FAILURE tests still emit nothing — they fail `can_fire` for OTHER reasons (no TU,
/// empty magazine, out of bounds), which a visible cell does not change.
fn place_enemy(app: &mut App, cell: CellLevel) -> Entity {
    let enemy = app.world_mut().spawn(ENEMY_FACTION).id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(enemy));
    // Mark the enemy cell squad-VISIBLE so the fire commit passes the GTW-11 fog gate.
    if let Some(fog) = app.world_mut().get_resource::<SquadVisibility>() {
        let mut visible: bevy::platform::collections::HashSet<CellLevel> =
            fog.visible_cells().copied().collect();
        let mut explored: bevy::platform::collections::HashSet<CellLevel> =
            fog.explored_cells().copied().collect();
        visible.insert(cell);
        explored.insert(cell);
        app.world_mut()
            .insert_resource(SquadVisibility::new(visible, explored));
    }
    enemy
}

/// The current `SelectedFireMode`.
fn fire_mode(app: &App) -> Option<FireModeSpec> {
    app.world().get_resource::<SelectedFireMode>().map(|m| **m)
}

// ---------------------------------------------------------------------------------
// AC1 — SelectedFireMode defaults to the selected weapon's single() on selecting an
// armed ganger, via the REAL selection path.
// ---------------------------------------------------------------------------------

/// AC1 — selecting an armed ganger sets `SelectedFireMode` to that weapon's
/// `FireMode::single()`.
#[test]
fn selecting_armed_ganger_defaults_fire_mode_to_single() {
    let mut app = acts_app();
    let selector = sbf_selector();
    let ganger = armed_ganger(
        &mut app,
        selector.clone(),
        StanceKind::Standing,
        Direction::North,
    );

    select_ganger(&mut app, ganger);

    assert_eq!(
        fire_mode(&app),
        Some(selector.single()),
        "selecting an armed ganger must default SelectedFireMode to its FireMode::single()",
    );
}

// ---------------------------------------------------------------------------------
// (GTW-254) The blind fire-mode cycle was REMOVED — the `gdtf_app` popup picker
// replaced it. Its old key-walk test (`fire_mode_cycle_walks_only_offered_modes`) is
// gone; the picker's open -> list-offered-modes -> select-sets-`SelectedFireMode`
// behavior is covered by the `gdtf_app` `action_bar.rs` integration tests on the real
// picker. AC1 (default-to-`single()` on select) above still covers
// `sync_fire_mode_on_select`, which the picker also relies on.
// ---------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------
// AC3 — a left-click on an in-bounds target emits exactly one FireRequested with the
// expected fields, when can_fire passes.
// ---------------------------------------------------------------------------------

/// AC3 — a left-click on an ENEMY target cell emits EXACTLY one `FireRequested { shooter
/// = *SelectedShooter, mode = *SelectedFireMode, target from the hovered cell }`, driven over
/// the REAL cursor -> `InspectTarget` -> `left_click_act` FIRE-branch chain.
#[test]
fn left_click_emits_one_fire_requested() {
    let mut app = acts_app();
    add_probes(&mut app);
    let selector = sbf_selector();
    let ganger = armed_ganger(
        &mut app,
        selector.clone(),
        StanceKind::Standing,
        Direction::North,
    );

    // Select the shooter (cursor over its own cell), then move the cursor to a distinct
    // in-grid target cell. The cursor stays put across the fire `update()`, so
    // `pick_hovered_cell` re-resolves the same target both before and after the
    // fire/select systems (ordering-independent).
    select_ganger(&mut app, ganger);
    let target = hover_at(&mut app, TARGET_CURSOR_OFFSET);
    let target_cell = target.cell();
    let target_level = Level::new(0);
    // GTW-238 FIRE requires an ENEMY occupant at the target — place one there.
    place_enemy(&mut app, target);

    press_left(&mut app);
    app.update();

    let emitted = fires(&app);
    assert_eq!(
        emitted.len(),
        1,
        "exactly one FireRequested must be emitted by a left-click on an in-bounds target",
    );
    let msg = &emitted[0];
    assert_eq!(msg.shooter, ganger, "shooter = *SelectedShooter");
    assert_eq!(msg.mode, selector.single(), "mode = *SelectedFireMode");
    assert_eq!(
        msg.target_cell, target_cell,
        "target cell from the hovered cell"
    );
    assert_eq!(
        msg.target_level, target_level,
        "target level from the hovered cell"
    );
}

// ---------------------------------------------------------------------------------
// AC4 — can_fire blocks emission: vary one failing input at a time -> zero
// FireRequested.
// ---------------------------------------------------------------------------------

/// AC4 — a Downed shooter, an empty magazine, insufficient TU each fail the shared
/// `can_fire` guard, and a cursor with NO in-bounds target (off-window) yields no target
/// — each causes NO `FireRequested`. Every case starts from the passing AC3 fixture and
/// breaks ONE input.
///
/// Note: the `can_fire` `in_bounds` predicate cannot fail through this surface — the
/// real `pick_hovered_cell` only ever resolves IN-grid cells (it is `None` off-grid), so
/// the "out-of-bounds target" failure manifests as "no in-bounds target -> no fire" (the
/// off-window cursor case). `can_fire`'s `in_bounds` is unit-tested directly in
/// `gdtf_battle_sim::magazine`.
#[test]
fn can_fire_failure_blocks_fire_requested() {
    // Downed shooter.
    {
        let mut app = acts_app();
        add_probes(&mut app);
        let ganger = armed_ganger(
            &mut app,
            sbf_selector(),
            StanceKind::Standing,
            Direction::North,
        );
        select_ganger(&mut app, ganger);
        let target = hover_at(&mut app, TARGET_CURSOR_OFFSET);
        place_enemy(&mut app, target);
        app.world_mut().entity_mut(ganger).insert(LifeState::Downed);
        press_left(&mut app);
        app.update();
        assert!(
            fires(&app).is_empty(),
            "a Downed shooter must emit no FireRequested"
        );
    }

    // Empty magazine.
    {
        let mut app = acts_app();
        add_probes(&mut app);
        let ganger = armed_ganger(
            &mut app,
            sbf_selector(),
            StanceKind::Standing,
            Direction::North,
        );
        select_ganger(&mut app, ganger);
        let target = hover_at(&mut app, TARGET_CURSOR_OFFSET);
        place_enemy(&mut app, target);
        // GTW-323 slice 3: the magazine lives on the related WEAPON entity now, so empty
        // THAT (the fire guard reads `ganger → Wields → weapon → Magazine`), not the ganger.
        empty_wielded_magazine(&mut app, ganger);
        press_left(&mut app);
        app.update();
        assert!(
            fires(&app).is_empty(),
            "an empty magazine must emit no FireRequested"
        );
    }

    // Insufficient TU — one below the selected mode's charge.
    {
        let mut app = acts_app();
        add_probes(&mut app);
        let ganger = armed_ganger(
            &mut app,
            sbf_selector(),
            StanceKind::Standing,
            Direction::North,
        );
        select_ganger(&mut app, ganger);
        let target = hover_at(&mut app, TARGET_CURSOR_OFFSET);
        place_enemy(&mut app, target);
        // The selected mode is single() (set on selection). Compute its exact charge and
        // set TU one below it via the SHARED mode_tu_cost source.
        let charge = gdtf_battle_sim::magazine::mode_tu_cost(
            &sbf_selector().single(),
            &TuMax::new(100),
            &Aiming::new(false),
            &CombatTuning::default(),
        );
        assert!(*charge > 0, "the mode charge must be positive for the test");
        app.world_mut()
            .entity_mut(ganger)
            .insert(Tu::new(charge.saturating_sub(1)));
        press_left(&mut app);
        app.update();
        assert!(
            fires(&app).is_empty(),
            "TU one below the mode charge must emit no FireRequested",
        );
    }

    // No in-bounds target — an off-window cursor resolves the hovered cell to `None`, so the
    // fire surface has no target and emits nothing (the in-bounds boundary at this layer).
    {
        let mut app = acts_app();
        add_probes(&mut app);
        let ganger = armed_ganger(
            &mut app,
            sbf_selector(),
            StanceKind::Standing,
            Direction::North,
        );
        select_ganger(&mut app, ganger);
        set_cursor(&mut app, None);
        app.update();
        assert_eq!(
            app.world()
                .get_resource::<InspectTarget>()
                .and_then(InspectTarget::hovered),
            None,
            "an off-window cursor must resolve the hovered cell to None",
        );
        press_left(&mut app);
        app.update();
        assert!(
            fires(&app).is_empty(),
            "no in-bounds target (off-window cursor) must emit no FireRequested",
        );
    }
}
