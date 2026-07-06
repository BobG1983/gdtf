//! The click-to-shot regression cases: the TU charge is observed end-to-end,
//! and the ranged weapon wins over a melee-first `Wields` order (GTW-289,
//! GTW-505 C5).

use bevy::prelude::*;
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{Direction, HeightBand, Magazine, OccupancyGrid, Tu};
use gdtf_test_utils::{clear_mouse, press_left};

use super::harness::*;

/// GTW-289 — a left-click on an ENEMY in a LEGITIMATE firing situation produces a SHOT
/// end-to-end: the shooter's TU strictly drops (the up-front mode charge) after the click.
#[test]
fn click_on_enemy_produces_a_shot_endtoend() {
    let mut app = endtoend_app();

    // Resolve the shooter cell + target cell from the real picker first, so the shooter's
    // Position / occupancy cell and the enemy's occupancy cell are the SAME cells the click
    // will resolve.
    let shooter_cell = hover_at(&mut app, SHOOTER_CURSOR_OFFSET);
    let target_cell = hover_at(&mut app, TARGET_CURSOR_OFFSET);

    // Place the shooter facing TOWARD the target (in-arc) so the simplest fire path runs;
    // ample TU also covers the out-of-arc turn-then-fire branch if the facing is off.
    let facing =
        Direction::from_cells(shooter_cell.cell(), target_cell.cell()).unwrap_or(Direction::East);
    let shooter = spawn_armed_shooter(&mut app, shooter_cell, facing);
    // Register the shooter in the occupancy grid at its own cell (so the SELECT click finds it)
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant(shooter_cell, Some(shooter));
        grid.set_occupant_band(shooter_cell, Some(HeightBand::High));
    }
    let enemy = place_armed_enemy(&mut app, target_cell);
    assert_ne!(shooter, enemy, "distinct shooter / enemy entities");

    // SELECT the shooter: cursor over its cell, fresh left-click, one update.
    let _ = hover_at(&mut app, SHOOTER_CURSOR_OFFSET);
    press_left(&mut app);
    app.update();
    clear_mouse(&mut app);
    assert_eq!(
        app.world()
            .get_resource::<SelectedShooter>()
            .and_then(|s| **s),
        Some(shooter),
        "the player-faction shooter must be SELECTED before firing",
    );

    // Move the cursor onto the ENEMY cell and snapshot the shooter's TU before the fire click.
    let _ = hover_at(&mut app, TARGET_CURSOR_OFFSET);
    let tu_before = app.world().get::<Tu>(shooter).map(|t| **t);
    assert!(tu_before.is_some(), "shooter has a Tu pool");

    // FIRE: a fresh left-click on the enemy cell. One update runs the WHOLE chain
    // (click -> intent -> drain -> FireRequested -> dispatch_fire -> fire()).
    press_left(&mut app);
    app.update();

    let tu_after = app.world().get::<Tu>(shooter).map(|t| **t);
    let shot_ran = matches!((tu_before, tu_after), (Some(b), Some(a)) if a < b);
    assert!(
        shot_ran,
        "clicking an enemy in a legitimate firing situation must run fire() (the shooter's \
         TU must drop by the mode charge) — tu {tu_before:?} -> {tu_after:?}. If TU is \
         UNCHANGED the fire path is silent end-to-end (GTW-289).",
    );

    // The selection must be UNCHANGED by a FIRE edge (GTW-238 — FIRE does not re-select).
    assert_eq!(
        app.world()
            .get_resource::<SelectedShooter>()
            .and_then(|s| **s),
        Some(shooter),
        "a FIRE edge leaves the selection untouched",
    );
}

/// GTW-505 C5 — the INPUT-LAYER zero-ranged-regression proof, ORDERING-INDEPENDENT: a shooter
/// wielding BOTH a melee weapon (related FIRST) AND a ranged weapon still fires the RANGED
/// weapon end-to-end. The input `can_fire` precheck (`fire_surface::try_fire_request`) resolves
/// the gun via `Wields::ranged_weapon` (the `MeleeWeapon`-marker filter), NOT `Wields::weapon`
/// (the FIRST related entity) — so the melee weapon being first never silences the fire.
///
/// PIN: with the OLD order-dependent `Wields::weapon()`, the FIRST related entity here is the
/// magazine-less melee weapon; `try_fire_request`'s `(Magazine, Handedness)` read would miss
/// it and fail closed → NO `FireRequested`, the shooter's TU UNCHANGED. Pinning the TU drop
/// (the mode charge) AND the RANGED magazine decrement proves the gun was resolved despite the
/// melee weapon sitting first in `Wields`.
#[test]
fn click_on_enemy_fires_the_ranged_weapon_even_with_a_melee_weapon_related_first() {
    let mut app = endtoend_app();

    let shooter_cell = hover_at(&mut app, SHOOTER_CURSOR_OFFSET);
    let target_cell = hover_at(&mut app, TARGET_CURSOR_OFFSET);

    let facing =
        Direction::from_cells(shooter_cell.cell(), target_cell.cell()).unwrap_or(Direction::East);
    // The MELEE weapon is related FIRST (so it is first in `Wields`), the ranged gun second.
    let shooter = spawn_armed_shooter_melee_first(&mut app, shooter_cell, facing);
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant(shooter_cell, Some(shooter));
        grid.set_occupant_band(shooter_cell, Some(HeightBand::High));
    }
    let enemy = place_armed_enemy(&mut app, target_cell);
    assert_ne!(shooter, enemy, "distinct shooter / enemy entities");

    // The ranged weapon's magazine BEFORE firing — resolved the ranged way (excluding the melee
    // weapon) so this reads the GUN's count, never the magazine-less melee entity.
    let rounds_before = ranged_magazine_rounds(&app, shooter);
    assert_eq!(
        rounds_before,
        Some(10),
        "precondition: the RANGED weapon (resolved excluding the melee one) holds 10 rounds",
    );

    // SELECT the shooter.
    let _ = hover_at(&mut app, SHOOTER_CURSOR_OFFSET);
    press_left(&mut app);
    app.update();
    clear_mouse(&mut app);
    assert_eq!(
        app.world()
            .get_resource::<SelectedShooter>()
            .and_then(|s| **s),
        Some(shooter),
        "the player-faction shooter must be SELECTED before firing",
    );

    // FIRE on the enemy.
    let _ = hover_at(&mut app, TARGET_CURSOR_OFFSET);
    let tu_before = app.world().get::<Tu>(shooter).map(|t| **t);
    press_left(&mut app);
    app.update();

    let tu_after = app.world().get::<Tu>(shooter).map(|t| **t);
    let shot_ran = matches!((tu_before, tu_after), (Some(b), Some(a)) if a < b);
    assert!(
        shot_ran,
        "with a melee weapon related FIRST, the input fire chain must STILL fire the ranged \
         weapon (the shooter's TU must drop) — tu {tu_before:?} -> {tu_after:?}. If TU is \
         UNCHANGED the input `can_fire` resolved the magazine-less melee weapon (the \
         order-dependent `Wields::weapon()` regression GTW-505 C5 guards).",
    );

    // The RANGED magazine decremented — proof the GUN was the entity `fire()` resolved + spent,
    // not the magazine-less melee weapon (a misresolution leaves rounds untouched).
    let rounds_after = ranged_magazine_rounds(&app, shooter);
    assert!(
        matches!((rounds_before, rounds_after), (Some(b), Some(a)) if a < b),
        "the RANGED magazine must drop (the burst spent it) — proof the gun, not the \
         magazine-less melee weapon, was fired: {rounds_before:?} -> {rounds_after:?}",
    );
}

/// The current round count of the shooter's RANGED weapon magazine, resolved the ranged way
/// (`Wields::ranged_weapon`, excluding the `MeleeWeapon`-marked entity) so it reads the GUN's
/// magazine even when a melee weapon is related first. `None` when unarmed / no ranged weapon.
fn ranged_magazine_rounds(app: &App, shooter: Entity) -> Option<u16> {
    use gdtf_battle_sim::{MeleeWeapon, Wields};
    let melee_entities: bevy::platform::collections::HashSet<Entity> = {
        let mut q = app
            .world()
            .try_query_filtered::<Entity, With<MeleeWeapon>>()?;
        q.iter(app.world()).collect()
    };
    app.world()
        .get::<Wields>(shooter)
        .and_then(|w| w.ranged_weapon(|e| melee_entities.contains(&e)))
        .and_then(|ranged| app.world().get::<Magazine>(ranged))
        .map(|m| *m.rounds())
}
