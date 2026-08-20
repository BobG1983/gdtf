//! An emplacement seeded from a def carries that def's sides and the spawn's facing.

use gdtf_battle_sim::{
    ganger::Direction,
    situation::CoverSpawn,
    terrain::{
        emplacement::{EmplacementEntrySides, EmplacementFacing},
        facing::TerrainFacing,
    },
    test_support::{SituationBuilder, test_terrain_registry},
};

use super::harness::*;

#[test]
fn a_seeded_emplacement_carries_its_defs_sides_unrotated_and_its_spawn_facing() {
    let (mut app, seed) = battle_app(0x5543_1186);
    let mut terrain = test_terrain_registry();
    terrain.insert(ONE_SIDED, one_sided_emplacement());
    app.insert_resource(terrain);

    let emp_cell = ground(6, 5);
    let situation = SituationBuilder::new()
        .with_gangers([player_at(ground(5, 5), Direction::East)])
        .with_scatter(CoverSpawn::new(emp_cell, ONE_SIDED, PLACED_FACING))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let emplacement = seated_emplacement(&mut app, emp_cell);

    assert_eq!(
        app.world()
            .get::<EmplacementEntrySides>(emplacement)
            .map(|sides| (**sides).clone()),
        Some(vec![AUTHORED_SIDE]),
        "the seeded entity carries the def's authored side UNROTATED — the reader rotates it, \
         seeding does not",
    );
    assert_eq!(
        app.world()
            .get::<EmplacementFacing>(emplacement)
            .map(|facing| **facing),
        Some(PLACED_FACING),
        "the seeded entity carries the facing read off its own CoverSpawn, not the default \
         facing {:?}",
        TerrainFacing::default(),
    );
}
