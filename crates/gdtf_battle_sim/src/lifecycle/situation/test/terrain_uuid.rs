//! pin-discriminating, asserting the def's authored kind / presence — never a balance
use bevy::asset::uuid::Uuid;

use super::support::*;

fn wall_def(key: TerrainUuid, graphic: &str) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new("Spec Wall".to_owned()),
        sim_kind: TerrainSimKind::Wall {
            hp:               CoverHp::new(150),
            armor_protection: ArmorProtection::new(9),
            armor_hardness:   ArmorHardness::new(5),
            height_band:      HeightBand::High,
        },
        presenter_kind: TerrainPresenterKind::Wall {
            graphic_name: TerrainGraphicKey::new(graphic.to_owned()),
        },
        tags: Vec::new(),
        on_death: None,

        blocks_pathing: None,
        blocks_los: None,
    }
}

fn slab_def(key: TerrainUuid, graphic: &str, footfall: Option<&str>) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new("Spec Slab".to_owned()),
        sim_kind: TerrainSimKind::Slab {
            hp:               crate::slab::SlabHp::new(90),
            armor_protection: ArmorProtection::new(3),
            armor_hardness:   ArmorHardness::new(1),
        },
        presenter_kind: TerrainPresenterKind::Slab {
            graphic_name: TerrainGraphicKey::new(graphic.to_owned()),
            footfall:     footfall.map(|s| FootfallSound::new(s.to_owned())),
        },
        tags: Vec::new(),
        on_death: None,

        blocks_pathing: None,
        blocks_los: None,
    }
}

#[test]
fn cover_uuid_resolves_against_def_registry_seeding_entry_and_kind() {
    let wall_at = key(2, 3, 0);
    let wall_key = TerrainUuid::new(Uuid::from_u128(0x0149_c0de_0000_0011));
    let registry = single_def_registry(wall_def(wall_key, "spec-wall"));

    let (mut situation, gangs) = SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .build_with_gangs();
    situation
        .walls
        .push(CoverSpawn::new(wall_at, wall_key, TerrainFacing::default()));

    let Some((mut app, _setup)) = run_setup_with(
        situation,
        gangs,
        test_registry(),
        test_armor_registry(),
        Some(&registry),
    ) else {
        return;
    };
    let world: &mut World = app.world_mut();

    let ledger = world.get_resource::<CoverLedger>();
    assert!(ledger.is_some(), "setup must insert the CoverLedger");
    let Some(ledger) = ledger else {
        return;
    };
    let entry = ledger.peek(&wall_at);
    assert!(
        entry.is_some(),
        "the resolved wall cell must have a CoverEntry seeded in the ledger",
    );
    let Some(entry) = entry else {
        return;
    };
    assert_eq!(
        entry.max_hp,
        CoverHp::new(150),
        "the seeded CoverEntry max HP must equal the resolved def's authored Wall hp",
    );
    assert_eq!(
        entry.height_band,
        HeightBand::High,
        "the seeded CoverEntry band must equal the resolved def's authored Wall band",
    );

    let grid = world.get_resource::<OccupancyGrid>();
    let Some(grid) = grid else {
        return;
    };
    assert_eq!(
        grid.terrain(&wall_at),
        TerrainKind::Wall,
        "a Wall-sim-kind def must seed TerrainKind::Wall into the occupancy grid",
    );
}

#[test]
fn wall_entity_carries_graphic_and_slab_carries_footfall() {
    let wall_at = key(4, 5, 0);
    let slab_at = key(6, 7, 1);
    let wall_key = TerrainUuid::new(Uuid::from_u128(0x0149_c0de_0000_0021));
    let slab_key = TerrainUuid::new(Uuid::from_u128(0x0149_c0de_0000_0022));

    let registry = TerrainDefRegistry::new([
        (wall_key, wall_def(wall_key, "spec-wall-graphic")),
        (
            slab_key,
            slab_def(slab_key, "spec-slab-graphic", Some("spec-step")),
        ),
    ]);

    let (mut situation, gangs) = SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .build_with_gangs();
    situation
        .walls
        .push(CoverSpawn::new(wall_at, wall_key, TerrainFacing::default()));
    situation
        .slabs
        .push(SlabSpawn::new(slab_at, slab_key, TerrainFacing::default()));

    let Some((mut app, _setup)) = run_setup_with(
        situation,
        gangs,
        test_registry(),
        test_armor_registry(),
        Some(&registry),
    ) else {
        return;
    };
    let world: &mut World = app.world_mut();

    let mut wall_query =
        world.query::<(&TerrainCell, &TerrainPieceKind, Option<&TerrainGraphicKey>)>();
    let wall_graphic = wall_query
        .iter(world)
        .find(|(cell, kind, _)| ***cell == wall_at && **kind == TerrainPieceKind::Wall)
        .and_then(|(_, _, graphic)| graphic.cloned());
    assert_eq!(
        wall_graphic,
        Some(TerrainGraphicKey::new("spec-wall-graphic".to_owned())),
        "the spawned WALL entity must carry the def's presenter graphic (NET-NEW: a \
         wall carried NO graphic on the old model)",
    );

    let mut slab_query = world.query::<(&TerrainCell, &TerrainPieceKind, Option<&FootfallSound>)>();
    let slab_footfall = slab_query
        .iter(world)
        .find(|(cell, kind, _)| ***cell == slab_at && **kind == TerrainPieceKind::Slab)
        .map(|(_, _, footfall)| footfall.cloned());
    assert_eq!(
        slab_footfall,
        Some(Some(FootfallSound::new("spec-step".to_owned()))),
        "the spawned SLAB entity must carry the def's optional footfall when it names one",
    );
}

fn blocking_slab_def(key: TerrainUuid, graphic: &str) -> TerrainDef {
    let mut def = slab_def(key, graphic, None);
    def.tags = vec![TerrainTag::BlocksPathfinding];
    def
}

#[test]
fn spawned_entities_carry_blocks_pathfinding_per_def() {
    let wall_at = key(4, 5, 0);
    let plain_slab_at = key(6, 7, 1);
    let barricade_slab_at = key(8, 9, 1);
    let wall_key = TerrainUuid::new(Uuid::from_u128(0x0149_c0de_0000_0031));
    let plain_slab_key = TerrainUuid::new(Uuid::from_u128(0x0149_c0de_0000_0032));
    let barricade_slab_key = TerrainUuid::new(Uuid::from_u128(0x0149_c0de_0000_0033));

    let registry = TerrainDefRegistry::new([
        (wall_key, wall_def(wall_key, "w")),
        (plain_slab_key, slab_def(plain_slab_key, "s", None)),
        (
            barricade_slab_key,
            blocking_slab_def(barricade_slab_key, "b"),
        ),
    ]);

    let (mut situation, gangs) = SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .build_with_gangs();
    situation
        .walls
        .push(CoverSpawn::new(wall_at, wall_key, TerrainFacing::default()));
    situation.slabs.push(SlabSpawn::new(
        plain_slab_at,
        plain_slab_key,
        TerrainFacing::default(),
    ));
    situation.slabs.push(SlabSpawn::new(
        barricade_slab_at,
        barricade_slab_key,
        TerrainFacing::default(),
    ));

    let Some((mut app, _setup)) = run_setup_with(
        situation,
        gangs,
        test_registry(),
        test_armor_registry(),
        Some(&registry),
    ) else {
        return;
    };
    let world: &mut World = app.world_mut();

    let has_marker = |world: &mut World, at: CellLevel| -> Option<bool> {
        let mut q = world.query::<(&TerrainCell, Option<&BlocksPathfinding>)>();
        q.iter(world)
            .find(|(cell, _)| ***cell == at)
            .map(|(_, marker)| marker.is_some())
    };

    assert_eq!(
        has_marker(world, wall_at),
        Some(true),
        "C1/D2: a Wall derives BlocksPathfinding by kind default (zero regression)",
    );
    assert_eq!(
        has_marker(world, plain_slab_at),
        Some(false),
        "C1/D2: a plain Slab does NOT carry BlocksPathfinding",
    );
    assert_eq!(
        has_marker(world, barricade_slab_at),
        Some(true),
        "C1: a Slab def with an explicit BlocksPathfinding tag DOES carry the marker",
    );
}
