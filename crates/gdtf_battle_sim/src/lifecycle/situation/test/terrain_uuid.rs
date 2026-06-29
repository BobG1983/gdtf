//! GTW-491 (T07a) — `setup_battle` resolves a [`CoverSpawn`] / [`SlabSpawn`] whose `piece`
//! is a [`TerrainUuid`] against a [`TerrainDefRegistry`], seeding the correct
//! [`CoverEntry`](crate::cover::CoverEntry) stats + occupancy [`TerrainKind`] (C1), and the
//! NET-NEW presenter-fact spawns — a WALL entity now carries a [`TerrainGraphicKey`] and a
//! SLAB entity carries the optional [`FootfallSound`] (C4).
//!
//! Every test drives the REAL [`setup_battle`] path end-to-end (no reimplementation) and is
//! pin-discriminating, asserting the def's authored kind / presence — never a balance
//! magnitude (the brittle-test rule).

use bevy::asset::uuid::Uuid;

use super::support::*;

/// A wall [`TerrainDef`] under the given UUID, carrying a Wall sim-kind + a Wall presenter
/// graphic (the NET-NEW wall-graphic fact). Arbitrary stats — the tests assert KIND /
/// presence, never magnitudes.
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
    }
}

/// A slab [`TerrainDef`] under the given UUID, carrying a Slab sim-kind + a Slab presenter
/// graphic and an OPTIONAL footfall.
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
    }
}

/// C1 (positive, pin-discriminating) — `setup_battle` resolves a `CoverSpawn` whose `piece`
/// is a [`TerrainUuid`] against a [`TerrainDefRegistry`], seeding the correct
/// [`CoverEntry`](crate::cover::CoverEntry) (the def's HP/band/armor) into the
/// [`CoverLedger`](crate::cover::CoverLedger) AND the def-variant-derived occupancy
/// [`TerrainKind::Wall`] into the [`OccupancyGrid`].
///
/// Pin-discriminating: the def is a `Wall` sim-kind, so its occupancy kind reads `Wall`
/// (re-deriving from list membership would also read `Wall` here — the cross-list C2 pin in
/// `occupancy_kind.rs` discriminates that). The `CoverEntry` HP/band are read back from the
/// ledger and compared to the def's OWN authored values (resolved through the real path), so
/// a wrong-UUID resolution (different def, or none) would mismatch.
#[test]
fn cover_uuid_resolves_against_def_registry_seeding_entry_and_kind() {
    let wall_at = key(2, 3, 0);
    let wall_key = TerrainUuid::new(Uuid::from_u128(0x0149_c0de_0000_0011));
    let registry = single_def_registry(wall_def(wall_key, "spec-wall"));

    let (mut situation, gangs) = SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .build_with_gangs();
    situation.walls.push(CoverSpawn::new(wall_at, wall_key));

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

    // The CoverLedger holds the wall cell, seeded with the DEF's HP ceiling + band (resolved
    // through the real path) — proving the UUID resolved to THIS def, not a default/empty.
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

    // The occupancy grid reads the def-variant-derived TerrainKind::Wall for the wall cell.
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

/// C4 (NET-NEW presenter fact, pin-discriminating) — a spawned WALL terrain entity now
/// carries a [`TerrainGraphicKey`] derived from the def's `presenter_kind.graphic_name` (on
/// the OLD model a Wall entity carried NO graphic), and a spawned SLAB entity carries the
/// optional [`FootfallSound`] when the def names one.
///
/// Pin-discriminating: the assertion is on the wall entity carrying SOME graphic equal to the
/// def's authored graphic name — a regression that stopped attaching a graphic to walls (the
/// old behaviour) reds the first assertion; a regression that dropped the slab footfall reds
/// the second.
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
    situation.walls.push(CoverSpawn::new(wall_at, wall_key));
    situation.slabs.push(SlabSpawn::new(slab_at, slab_key));

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

    // Find the WALL terrain entity (by cell + Wall piece kind) and assert it carries the
    // def's graphic — the GTW-491 NET-NEW fact (a wall had no graphic on the old model).
    let mut wall_query =
        world.query::<(&TerrainCell, &TerrainPieceKind, Option<&TerrainGraphicKey>)>();
    let wall_graphic = wall_query
        .iter(world)
        .find(|(cell, kind, _)| ***cell == wall_at && **kind == TerrainPieceKind::Wall)
        .and_then(|(_, _, graphic)| graphic.cloned());
    assert_eq!(
        wall_graphic,
        Some(TerrainGraphicKey::new("spec-wall-graphic".to_owned())),
        "the spawned WALL entity must carry the def's presenter graphic (GTW-491 NET-NEW: a \
         wall carried NO graphic on the old model)",
    );

    // Find the SLAB terrain entity (by cell + Slab piece kind) and assert it carries the
    // def's optional footfall.
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
