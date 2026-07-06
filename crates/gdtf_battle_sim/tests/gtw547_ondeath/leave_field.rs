//! The leave-field-on-death effect: destroyed cover and a melee-smashed barrel
//! spawn the referenced field at the death cell.

use bevy::app::App;
use gdtf_battle_sim::{
    acts::{FireRequested, MeleeRequested},
    effects::fields::FieldRegistry,
    ganger::Direction,
    metric::{Cell, CellLevel, Level},
    situation::CoverSpawn,
    test_support::{SituationBuilder, single_mode},
};

use super::harness::*;

// === `LeaveField` headline: destroyed cover leaves the referenced field at its cell. ===

#[test]
fn destroyed_cover_with_leave_field_spawns_the_field_at_that_cell() {
    let (mut app, seed) = battle_app(0x5547_0B0B, true);

    // Shooter at (5,5) facing East; a 1-HP fuel BARREL (Cover with a `LeaveField` on-death) at
    // (8,5). Aiming at the barrel cell destroys it in one shot; its destruction leaves the
    // `burning` field there.
    let situation = SituationBuilder::new()
        .with_gangers([shooter(ground(5, 5), Direction::East)])
        .with_scatter(CoverSpawn::new(ground(8, 5), BARREL))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let Some(shooter_e) = ganger_at(&mut app, ground(5, 5)) else {
        unreachable!("setup spawns the shooter at (5,5)");
    };

    // Before the shot: no field at the barrel cell.
    assert!(
        !field_present(&app, ground(8, 5)),
        "no field exists at the barrel cell before it is destroyed"
    );

    // Fire at the barrel cell — the shot destroys the 1-HP cover, emitting CoverDestroyed +
    // OnDeathOccurred(cover); resolve_on_death then spawns the `burning` field there.
    app.world_mut().write_message(FireRequested::new(
        shooter_e,
        single_mode(0.2, 1),
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 4);

    // The referenced field now exists at the destroyed barrel's cell (persists per GTW-545).
    assert!(
        field_present(&app, ground(8, 5)),
        "destroying the barrel left the `burning` field at its cell (the `LeaveField` effect)"
    );
}

/// Whether a live field sits at `at` in the battle-lifetime [`FieldRegistry`].
fn field_present(app: &App, at: CellLevel) -> bool {
    app.world()
        .get_resource::<FieldRegistry>()
        .is_some_and(|r| r.field_at(&at).is_some())
}

// === Melee cover-smash gate: a melee-smashed barrel leaves its on-death field. ===

#[test]
fn a_barrel_smashed_in_melee_leaves_its_on_death_field() {
    let (mut app, seed) = battle_app(0x5547_0D0D, true);
    // A lethal melee weapon so one smash destroys the 1-HP barrel (the cover-smash kill gate).
    app.world_mut().insert_resource(lethal_melee_registry());

    // A faction-0 attacker at (5,5) facing East; a 1-HP fuel BARREL (Cover with a `LeaveField`
    // on-death) at (6,5) — 8-adjacent, so the melee smash destroys it, leaving the field.
    let situation = SituationBuilder::new()
        .with_gangers([melee_attacker(ground(5, 5), PLAYER, Direction::East)])
        .with_scatter(CoverSpawn::new(ground(6, 5), BARREL))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let Some(attacker) = ganger_at(&mut app, ground(5, 5)) else {
        unreachable!("setup spawns the attacker at (5,5)");
    };

    // Before the smash: no field at the barrel cell.
    assert!(
        !field_present(&app, ground(6, 5)),
        "no field exists at the barrel cell before it is smashed"
    );

    // Smash the barrel in melee THROUGH the buffered structural MeleeRequested. The smash
    // destroys the 1-HP cover, emitting OnDeathOccurred(cover); resolve_on_death then spawns
    // the `burning` field there.
    app.world_mut()
        .write_message(MeleeRequested::new_structural(attacker, ground(6, 5)));
    step(&mut app, 4);

    // The referenced field now exists at the smashed barrel's cell (the cover-smash gate).
    // PIN-DISCRIMINATING: reverting the melee cover-smash `deaths.write(...)` leaves no field.
    assert!(
        field_present(&app, ground(6, 5)),
        "smashing the barrel in melee left the `burning` field at its cell (the cover-smash \
         terminal-death gate)"
    );
}
