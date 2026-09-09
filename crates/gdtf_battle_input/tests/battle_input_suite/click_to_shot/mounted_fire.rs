//! Clicking while manning a mount: the pointer path acts with the MOUNT, not the carried gun.

use bevy::prelude::*;
use cobalt_test_utils::{clear_mouse, press_left};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    cover::HeightBand,
    prelude::{CellLevel, Direction, OccupancyGrid, Tu},
};

use super::harness::*;

/// Rounds the probe gun and the probe turret spawn with.
const LOADED: u16 = 10;

#[test]
fn click_on_enemy_costs_nothing_when_the_mounted_weapon_is_dry() {
    let mut app = endtoend_app();

    let shooter_cell = hover_at(&mut app, SHOOTER_CURSOR_OFFSET);
    let target_cell = hover_at(&mut app, TARGET_CURSOR_OFFSET);

    // Facing away, so a shot the input layer lets through costs the turn before it bails.
    let away =
        Direction::from_cells(target_cell.cell(), shooter_cell.cell()).unwrap_or(Direction::West);
    let shooter = spawn_armed_shooter(&mut app, shooter_cell, away);
    let turret = mount_turret(&mut app, shooter, 0);
    stand_at(&mut app, shooter, shooter_cell);
    let enemy = place_armed_enemy(&mut app, target_cell);
    assert_ne!(shooter, enemy, "distinct shooter / enemy entities");

    let carried = carried_gun(&mut app, shooter);
    assert!(carried.is_some(), "the shooter must wield a carried gun");
    let Some(gun) = carried else { return };
    assert_eq!(
        magazine_rounds(&app, gun),
        Some(LOADED),
        "precondition: the CARRIED gun is loaded, so only the dry mount can refuse the shot",
    );
    assert_eq!(
        magazine_rounds(&app, turret),
        Some(0),
        "precondition: the MOUNTED weapon is dry",
    );

    select_shooter(&mut app, shooter);

    let _ = hover_at(&mut app, TARGET_CURSOR_OFFSET);
    let tu_before = app.world().get::<Tu>(shooter).map(|t| **t);
    assert!(tu_before.is_some(), "shooter has a Tu pool");

    press_left(&mut app);
    app.update();

    assert_eq!(
        app.world().get::<Tu>(shooter).map(|t| **t),
        tu_before,
        "with the MOUNTED weapon dry the click must cost NOTHING — the pointer fire path must \
         price the weapon dispatch_fire will fire, not the loaded gun still in the shooter's \
         hands, or the turn into arc is charged for a shot that never happens",
    );
    assert_eq!(
        magazine_rounds(&app, gun),
        Some(LOADED),
        "the CARRIED gun must be untouched — nothing fired at all",
    );

    // Control: the same click in the same pose fires once the MOUNT has rounds.
    refill_magazine(&mut app, turret);
    let tu_loaded = app.world().get::<Tu>(shooter).map(|t| **t);
    press_left(&mut app);
    app.update();

    assert!(
        matches!((tu_loaded, app.world().get::<Tu>(shooter).map(|t| **t)), (Some(b), Some(a)) if a < b),
        "control: with the MOUNT loaded the very same click must fire (TU drops), so the refusal \
         above came from the mount and not from the pose",
    );
}

#[test]
fn click_on_enemy_fires_the_mount_while_the_carried_gun_is_dry() {
    let mut app = endtoend_app();

    let shooter_cell = hover_at(&mut app, SHOOTER_CURSOR_OFFSET);
    let target_cell = hover_at(&mut app, TARGET_CURSOR_OFFSET);

    let facing =
        Direction::from_cells(shooter_cell.cell(), target_cell.cell()).unwrap_or(Direction::East);
    let shooter = spawn_armed_shooter(&mut app, shooter_cell, facing);
    let turret = mount_turret(&mut app, shooter, LOADED);
    stand_at(&mut app, shooter, shooter_cell);
    let enemy = place_armed_enemy(&mut app, target_cell);
    assert_ne!(shooter, enemy, "distinct shooter / enemy entities");

    let carried = carried_gun(&mut app, shooter);
    assert!(carried.is_some(), "the shooter must wield a carried gun");
    let Some(gun) = carried else { return };
    empty_magazine(&mut app, gun);
    assert_eq!(
        magazine_rounds(&app, gun),
        Some(0),
        "precondition: the CARRIED gun is dry",
    );
    assert_eq!(
        magazine_rounds(&app, turret),
        Some(LOADED),
        "precondition: the MOUNTED weapon is loaded",
    );

    select_shooter(&mut app, shooter);

    let _ = hover_at(&mut app, TARGET_CURSOR_OFFSET);
    let tu_before = app.world().get::<Tu>(shooter).map(|t| **t);
    press_left(&mut app);
    app.update();

    assert!(
        matches!((tu_before, app.world().get::<Tu>(shooter).map(|t| **t)), (Some(b), Some(a)) if a < b),
        "the shot must go: the dry gun in the shooter's hands is not the weapon that fires, so \
         it cannot refuse the click — tu {tu_before:?}",
    );
    let turret_after = magazine_rounds(&app, turret);
    assert!(
        matches!(turret_after, Some(rounds) if rounds < LOADED),
        "the MOUNT's rounds are the ones spent, so the selection ran mount-first and not \
         first-non-melee: {turret_after:?} of {LOADED}",
    );
}

fn select_shooter(app: &mut App, shooter: Entity) {
    let _ = hover_at(app, SHOOTER_CURSOR_OFFSET);
    press_left(app);
    app.update();
    clear_mouse(app);
    assert_eq!(
        app.world()
            .get_resource::<SelectedShooter>()
            .and_then(|s| **s),
        Some(shooter),
        "the player-faction shooter must be SELECTED before firing",
    );
}

fn stand_at(app: &mut App, shooter: Entity, cell: CellLevel) {
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant(cell, Some(shooter));
        grid.set_occupant_band(cell, Some(HeightBand::High));
    }
}
