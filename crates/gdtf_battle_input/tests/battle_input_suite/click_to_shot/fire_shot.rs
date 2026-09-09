use bevy::prelude::*;
use cobalt_test_utils::{clear_mouse, press_left};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    cover::HeightBand,
    magazine::Magazine,
    prelude::{Direction, OccupancyGrid, Tu},
};

use super::harness::*;

#[test]
fn click_on_enemy_produces_a_shot_endtoend() {
    let mut app = endtoend_app();

    let shooter_cell = hover_at(&mut app, SHOOTER_CURSOR_OFFSET);
    let target_cell = hover_at(&mut app, TARGET_CURSOR_OFFSET);

    let facing =
        Direction::from_cells(shooter_cell.cell(), target_cell.cell()).unwrap_or(Direction::East);
    let shooter = spawn_armed_shooter(&mut app, shooter_cell, facing);
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant(shooter_cell, Some(shooter));
        grid.set_occupant_band(shooter_cell, Some(HeightBand::High));
    }
    let enemy = place_armed_enemy(&mut app, target_cell);
    assert_ne!(shooter, enemy, "distinct shooter / enemy entities");

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

    let _ = hover_at(&mut app, TARGET_CURSOR_OFFSET);
    let tu_before = app.world().get::<Tu>(shooter).map(|t| **t);
    assert!(tu_before.is_some(), "shooter has a Tu pool");

    press_left(&mut app);
    app.update();

    let tu_after = app.world().get::<Tu>(shooter).map(|t| **t);
    let shot_ran = matches!((tu_before, tu_after), (Some(b), Some(a)) if a < b);
    assert!(
        shot_ran,
        "clicking an enemy in a legitimate firing situation must run fire() (the shooter's \
         TU must drop by the mode charge) — tu {tu_before:?} -> {tu_after:?}. If TU is \
         UNCHANGED the fire path is silent end-to-end.",
    );

    assert_eq!(
        app.world()
            .get_resource::<SelectedShooter>()
            .and_then(|s| **s),
        Some(shooter),
        "a FIRE edge leaves the selection untouched",
    );
}

#[test]
fn click_on_enemy_fires_the_ranged_weapon_even_with_a_melee_weapon_related_first() {
    let mut app = endtoend_app();

    let shooter_cell = hover_at(&mut app, SHOOTER_CURSOR_OFFSET);
    let target_cell = hover_at(&mut app, TARGET_CURSOR_OFFSET);

    let facing =
        Direction::from_cells(shooter_cell.cell(), target_cell.cell()).unwrap_or(Direction::East);
    let shooter = spawn_armed_shooter_melee_first(&mut app, shooter_cell, facing);
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant(shooter_cell, Some(shooter));
        grid.set_occupant_band(shooter_cell, Some(HeightBand::High));
    }
    let enemy = place_armed_enemy(&mut app, target_cell);
    assert_ne!(shooter, enemy, "distinct shooter / enemy entities");

    let rounds_before = ranged_magazine_rounds(&app, shooter);
    assert_eq!(
        rounds_before,
        Some(10),
        "precondition: the RANGED weapon (resolved excluding the melee one) holds 10 rounds",
    );

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
         order-dependent `Wields::weapon()` regression C5 guards).",
    );

    let rounds_after = ranged_magazine_rounds(&app, shooter);
    assert!(
        matches!((rounds_before, rounds_after), (Some(b), Some(a)) if a < b),
        "the RANGED magazine must drop (the burst spent it) — proof the gun, not the \
         magazine-less melee weapon, was fired: {rounds_before:?} -> {rounds_after:?}",
    );
}

fn ranged_magazine_rounds(app: &App, shooter: Entity) -> Option<u16> {
    use gdtf_battle_sim::weapon::{MeleeWeapon, Wields};
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
