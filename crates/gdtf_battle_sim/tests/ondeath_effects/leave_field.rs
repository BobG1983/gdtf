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

#[test]
fn destroyed_cover_with_leave_field_spawns_the_field_at_that_cell() {
    let (mut app, seed) = battle_app(0x5547_0B0B, true);

    let situation = SituationBuilder::new()
        .with_gangers([shooter(ground(5, 5), Direction::East)])
        .with_scatter(CoverSpawn::new(ground(8, 5), BARREL))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let Some(shooter_e) = ganger_at(&mut app, ground(5, 5)) else {
        unreachable!("setup spawns the shooter at (5,5)");
    };

    assert!(
        !field_present(&app, ground(8, 5)),
        "no field exists at the barrel cell before it is destroyed"
    );

    app.world_mut().write_message(FireRequested::new(
        shooter_e,
        single_mode(0.2, 1),
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 4);

    assert!(
        field_present(&app, ground(8, 5)),
        "destroying the barrel left the `burning` field at its cell (the `LeaveField` effect)"
    );
}

fn field_present(app: &App, at: CellLevel) -> bool {
    app.world()
        .get_resource::<FieldRegistry>()
        .is_some_and(|r| r.field_at(&at).is_some())
}

#[test]
fn a_barrel_smashed_in_melee_leaves_its_on_death_field() {
    let (mut app, seed) = battle_app(0x5547_0D0D, true);
    app.world_mut().insert_resource(lethal_melee_registry());

    let situation = SituationBuilder::new()
        .with_gangers([melee_attacker(ground(5, 5), PLAYER, Direction::East)])
        .with_scatter(CoverSpawn::new(ground(6, 5), BARREL))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let Some(attacker) = ganger_at(&mut app, ground(5, 5)) else {
        unreachable!("setup spawns the attacker at (5,5)");
    };

    assert!(
        !field_present(&app, ground(6, 5)),
        "no field exists at the barrel cell before it is smashed"
    );

    app.world_mut()
        .write_message(MeleeRequested::new_structural(attacker, ground(6, 5)));
    step(&mut app, 4);

    assert!(
        field_present(&app, ground(6, 5)),
        "smashing the barrel in melee left the `burning` field at its cell (the cover-smash \
         terminal-death gate)"
    );
}
