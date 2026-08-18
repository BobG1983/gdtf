//! An emplacement seeded from a def carries that def's sides and the spawn's facing.

use bevy::asset::uuid::Uuid;
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    ganger::Direction,
    situation::CoverSpawn,
    terrain::{
        def::{TerrainDef, TerrainDisplayName, TerrainPresenterKind, TerrainSimKind, TerrainUuid},
        emplacement::{EmplacementEntrySides, EmplacementFacing},
        facing::TerrainFacing,
        piece::TerrainGraphicKey,
    },
    test_support::{SituationBuilder, TEST_MOUNTED_WEAPON_KEY, test_terrain_registry},
    weapon::WeaponName,
};

use super::harness::*;

const ONE_SIDED: TerrainUuid = TerrainUuid::new(Uuid::from_u128(0x0149_1491_1186_0001));

const AUTHORED_SIDE: TerrainFacing = TerrainFacing::North;

const PLACED_FACING: TerrainFacing = TerrainFacing::East;

fn one_sided_emplacement() -> TerrainDef {
    TerrainDef {
        key:            ONE_SIDED,
        display_name:   TerrainDisplayName::new("One-Sided Mount".to_owned()),
        sim_kind:       TerrainSimKind::Emplacement {
            hp:               CoverHp::new(45),
            armor_protection: ArmorProtection::new(0),
            armor_hardness:   ArmorHardness::new(0),
            height_band:      HeightBand::High,
            mounted_weapon:   WeaponName::new(TEST_MOUNTED_WEAPON_KEY.to_owned()),
            entry_sides:      vec![AUTHORED_SIDE],
        },
        presenter_kind: TerrainPresenterKind::Emplacement {
            graphic_name: TerrainGraphicKey::new("emplacement".to_owned()),
        },
        tags:           Vec::new(),
        on_death:       None,
        blocks_pathing: None,
        blocks_los:     None,
    }
}

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
