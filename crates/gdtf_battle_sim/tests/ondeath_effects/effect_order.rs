use gdtf_battle_sim::{
    acts::FireRequested,
    ganger::Direction,
    metric::{Cell, Level},
    situation::CoverSpawn,
    terrain::facing::TerrainFacing,
    test_support::{SituationBuilder, single_mode},
};

use super::{fixtures::*, harness::*};

#[test]
fn a_def_authoring_two_effects_fires_both_at_the_destroyed_cell() {
    let (mut app, seed) = battle_app(0x5547_1A1A, false);
    app.insert_resource(ordered_cover_registry(vec![
        explode_effect(),
        leave_field_effect(burning()),
    ]));
    insert_field_defs(&mut app);

    let situation = SituationBuilder::new()
        .with_gangers([
            shooter(ground(5, 5), Direction::East),
            bystander(ground(8, 4)),
        ])
        .with_scatter(CoverSpawn::new(
            ground(8, 5),
            ORDERED_COVER,
            TerrainFacing::default(),
        ))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let (Some(shooter_e), Some(neighbour)) = (
        ganger_at(&mut app, ground(5, 5)),
        ganger_at(&mut app, ground(8, 4)),
    ) else {
        unreachable!("setup spawns the shooter at (5,5) and the neighbour at (8,4)");
    };
    let neighbour0 = vitals(&app, neighbour);
    assert!(
        field_def_at(&app, ground(8, 5)).is_none(),
        "PRECONDITION: no field stands at the cover cell before it is destroyed",
    );

    app.world_mut().write_message(FireRequested::new(
        shooter_e,
        single_mode(0.2, 1),
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 4);

    assert!(
        took_damage(neighbour0, vitals(&app, neighbour)),
        "the list's first effect fanned: the adjacent ganger took the Explode's blast (before \
         {neighbour0:?}, after {:?})",
        vitals(&app, neighbour),
    );
    assert_eq!(
        field_def_at(&app, ground(8, 5)),
        Some(burning_def()),
        "the list's second effect fanned: the burning def stands at the destroyed cell",
    );
}

#[test]
fn the_later_leave_field_is_the_one_the_cell_keeps() {
    let (mut app, seed) = battle_app(0x5547_1B1B, false);
    app.insert_resource(ordered_cover_registry(vec![
        leave_field_effect(burning()),
        leave_field_effect(toxic()),
    ]));
    insert_field_defs(&mut app);

    let situation = SituationBuilder::new()
        .with_gangers([shooter(ground(5, 5), Direction::East)])
        .with_scatter(CoverSpawn::new(
            ground(8, 5),
            ORDERED_COVER,
            TerrainFacing::default(),
        ))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let Some(shooter_e) = ganger_at(&mut app, ground(5, 5)) else {
        unreachable!("setup spawns the shooter at (5,5)");
    };
    assert!(
        field_def_at(&app, ground(8, 5)).is_none(),
        "PRECONDITION: no field stands at the cover cell before it is destroyed",
    );

    app.world_mut().write_message(FireRequested::new(
        shooter_e,
        single_mode(0.2, 1),
        Cell::new(8, 5),
        Level::new(0),
    ));
    step(&mut app, 4);

    assert_eq!(
        field_def_at(&app, ground(8, 5)).map(|def| def.damage_type),
        Some(toxic_def().damage_type),
        "both LeaveField effects fanned at the one cell, so the cell keeps the LAST authored \
         def; it holds {:?}",
        field_def_at(&app, ground(8, 5)),
    );
}
