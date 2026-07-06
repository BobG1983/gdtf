//! The positive populate paths: a fireable enemy / the weapon-derived cost /
//! shootable cover (C2/C4b, GTW-376, GTW-377).

use gdtf_battle_input::SelectedFireMode;
use gdtf_battle_sim::{
    magazine::mode_tu_cost,
    prelude::{Cell, CellLevel},
    tuning::CombatTuning,
};

use super::harness::*;

/// C2 / C4b (positive) — hovering a fireable enemy populates the highlight with the hovered cell +
/// a cost EXACTLY equal to `mode_tu_cost` (computed independently in-test, no magnitude pin).
#[test]
fn hovering_fireable_enemy_populates_cell_and_mode_tu_cost() {
    let mut app = fire_target_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let enemy_cell = CellLevel::new(Cell::new(13, 11), LEVEL);
    let mode = spec(0.2);

    let (_shooter, tu_max, aiming) = spawn_and_select_shooter(&mut app, shooter_cell);
    place_enemy(&mut app, enemy_cell);
    set_fire_mode(&mut app, mode);
    set_hovered(&mut app, Some(enemy_cell));

    // The expected cost, computed the SAME way the shot will charge it (REUSE, no magnitude pin).
    let tuning = app.world().resource::<CombatTuning>();
    let expected_cost = mode_tu_cost(&mode, &tu_max, &aiming, tuning);

    app.update();

    let h = highlight(&app);
    assert_eq!(
        h.cell(),
        Some(enemy_cell),
        "hovering a fireable enemy populates the highlight with the hovered cell (13,11,L0)",
    );
    assert_eq!(
        h.cost(),
        Some(expected_cost),
        "the highlight cost EXACTLY equals mode_tu_cost(SelectedFireMode, TuMax, Aiming, tuning)",
    );
}

/// GTW-376 DEFECT 1 (pin-discriminating) — hovering a fireable VISIBLE enemy with the
/// `SelectedFireMode` resolved off the WEAPON via the REAL in-app `sync_fire_mode_on_select`
/// path (NOT an injected mode) populates the highlight with a cost EXACTLY equal to
/// `mode_tu_cost(resolved mode, TuMax, Aiming, tuning)` AND strictly GREATER THAN ZERO.
///
/// This reproduces the runtime bug: the wielded weapon entity spawns a frame AFTER the
/// battle-start auto-select flips the selection (the deferred `queue_spawn_related_scenes`
/// race), so `sync_fire_mode_on_select`'s `selected.is_changed()` branch has already passed
/// when the weapon's `FireMode` becomes queryable — leaving `SelectedFireMode` at the
/// `tu_percent: 0` default and the cost reading "0 TU". The GTW-376 `Added<FireMode>`
/// re-trigger recovers the mode the frame the weapon arrives. The `> 0` assertion FAILS if
/// the mode stays at the zero default (the exact in-engine symptom).
#[test]
fn fireable_enemy_cost_resolves_off_weapon_and_is_nonzero() {
    let mut app = fire_target_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let enemy_cell = CellLevel::new(Cell::new(13, 11), LEVEL);

    // Arm via the REAL resolution path: select, then spawn the weapon's FireMode LATE
    // (the deferred-spawn race) with a non-zero authored TU%. `sync_fire_mode_on_select` must
    // recover the mode off the late-spawned weapon.
    let (_shooter, tu_max, aiming, _) = spawn_select_then_arm_late(&mut app, shooter_cell, 0.3);
    place_enemy(&mut app, enemy_cell);
    set_hovered(&mut app, Some(enemy_cell));

    // The resolved SelectedFireMode (off the weapon) is what the producer + the shot both read.
    // The expected cost is computed from this resolved mode — so if the GTW-376 race left it at
    // the `tu_percent: 0` default, `expected_cost` is 0 and the `> 0` assertion below FAILS (no
    // magnitude pin: the cost is whatever `mode_tu_cost` yields, asserted `> 0` separately).
    let resolved_mode = **app.world().resource::<SelectedFireMode>();
    let tuning = app.world().resource::<CombatTuning>();
    let expected_cost = mode_tu_cost(&resolved_mode, &tu_max, &aiming, tuning);

    app.update();

    let h = highlight(&app);
    assert_eq!(
        h.cell(),
        Some(enemy_cell),
        "hovering a fireable VISIBLE enemy populates the highlight with the hovered cell",
    );
    assert_eq!(
        h.cost(),
        Some(expected_cost),
        "the highlight cost EXACTLY equals mode_tu_cost(resolved SelectedFireMode, TuMax, \
         Aiming, tuning)",
    );
    // PIN-DISCRIMINATING: a normal weapon/mode charges a non-zero cost — this FAILS if the mode
    // is stuck at the tu_percent:0 default (the "0 TU" in-engine defect).
    assert!(
        h.cost().is_some_and(|cost| *cost > 0),
        "the resolved fire cost is strictly positive for a normal weapon/mode (not 0 TU); got {:?}",
        h.cost(),
    );
}

// ---------------------------------------------------------------------------------
// GTW-377 — shootable COVER / WALL is a valid fire target: the highlight populates over a
// hovered cover cell with the SAME treatment (cell + the shot's `mode_tu_cost`).
// ---------------------------------------------------------------------------------

/// GTW-377 C1 / C2 / C6a / C6c (positive) — hovering a SHOOTABLE cover/wall cell (no occupant,
/// blocking, squad-VISIBLE) populates the presenter-owned highlight with the hovered cover cell
/// and a cost EXACTLY equal to `mode_tu_cost` — the SAME treatment a fireable enemy gets (the
/// red-tile + TU-cost highlight the presenter draws). The discriminator recognizes the cover
/// cell as a valid fire target (C1) and the highlight read-seam is positively populated for it
/// (C6c). No magnitude pin — the cost is computed independently in-test.
#[test]
fn hovering_shootable_cover_populates_cell_and_mode_tu_cost() {
    let mut app = fire_target_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let cover_cell = CellLevel::new(Cell::new(13, 11), LEVEL);
    let mode = spec(0.2);

    let (_shooter, tu_max, aiming) = spawn_and_select_shooter(&mut app, shooter_cell);
    // A cover cell — blocking structure, NO occupant, squad-VISIBLE.
    place_cover(&mut app, cover_cell);
    set_fire_mode(&mut app, mode);
    set_hovered(&mut app, Some(cover_cell));

    // The expected cost, computed the SAME way the shot will charge it (REUSE, no magnitude pin).
    let tuning = app.world().resource::<CombatTuning>();
    let expected_cost = mode_tu_cost(&mode, &tu_max, &aiming, tuning);

    app.update();

    let h = highlight(&app);
    assert_eq!(
        h.cell(),
        Some(cover_cell),
        "hovering a shootable cover cell populates the highlight with the hovered cover cell \
         (13,11,L0) — cover is a valid fire target (C1)",
    );
    assert_eq!(
        h.cost(),
        Some(expected_cost),
        "the cover-target highlight cost EXACTLY equals mode_tu_cost(SelectedFireMode, TuMax, \
         Aiming, tuning) — same TU-cost treatment as a ganger target (C2)",
    );
}
