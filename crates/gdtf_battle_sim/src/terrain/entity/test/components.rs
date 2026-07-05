//! Tests 1 + 2 — one entity per terrain piece (C1) + authored/prototype static
//! stats on the spawned entities (C2).

use super::support::*;
use crate::{
    armor::{ArmorHardness, ArmorProtection},
    battle::SetupBattleRequested,
    cover::{CoverHp, HeightBand},
    slab::SlabHp,
    terrain::entity::{TerrainCell, TerrainIndex, TerrainIndexKey},
    test_support::{SituationBuilder, ganger_at},
};

// ── Test 1 — One entity per terrain piece spawned (C1) ───────────────────────

/// `setup_battle` spawns exactly N+M+K [`TerrainCell`] entities for N walls,
/// M scatter props, and K slabs.
#[test]
fn test1_one_entity_per_terrain_piece() {
    let mut app = headless_app();

    // 1 wall (N=1), 0 scatter (M=0), 1 slab (K=1) → expect 2 entities.
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(cl(0, 0, 0), 0))
        .with_ganger(ganger_at(cl(1, 1, 0), 1))
        .wall_at(cl(2, 2, 0))
        .slab_at(cl(3, 3, 1))
        .build();

    app.world_mut().write_message(SetupBattleRequested::new(
        situation,
        crate::rng::BattleSeed::new(0xAB),
    ));
    app.update();

    let world = app.world_mut();
    let mut q = world.query::<&TerrainCell>();
    assert_eq!(
        q.iter(world).count(),
        2,
        "Test 1 (C1): exactly 1 wall + 1 slab = 2 TerrainCell entities",
    );
}

/// More terrain: 2 walls + 1 scatter + 2 slabs = 5 entities.
#[test]
fn test1_counts_all_kinds() {
    use crate::situation::CoverSpawn;

    let mut app = headless_app();

    let scatter = CoverSpawn::new(
        cl(9, 9, 0),
        // GTW-491: def UUID resolved against the test terrain registry; COVER = LOW-band
        // Cover (the def's sim-kind determines what entities count — this test asserts
        // entity count, not band).
        crate::test_support::test_pieces::COVER,
    );
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(cl(0, 0, 0), 0))
        .with_ganger(ganger_at(cl(1, 1, 0), 1))
        .wall_at(cl(2, 2, 0))
        .wall_at(cl(3, 3, 0))
        .with_scatter(scatter)
        .slab_at(cl(4, 4, 1))
        .slab_at(cl(5, 5, 1))
        .build();

    app.world_mut().write_message(SetupBattleRequested::new(
        situation,
        crate::rng::BattleSeed::new(0xCD),
    ));
    app.update();

    let world = app.world_mut();
    let mut q = world.query::<&TerrainCell>();
    assert_eq!(
        q.iter(world).count(),
        5,
        "Test 1: 2 walls + 1 scatter + 2 slabs = 5 TerrainCell entities",
    );
}

// ── Test 2 — Entity carries static stats (C2) ────────────────────────────────

/// A known wall entity carries `CoverHp`, `HeightBand`, `ArmorProtection`,
/// `ArmorHardness` components matching the authored `CoverSpawn`.
#[test]
fn test2_cover_entity_carries_authored_stats() {
    let wall_cell = cl(2, 2, 0);
    let authored_hp = CoverHp::new(120);
    let authored_band = HeightBand::High;
    let authored_prot = ArmorProtection::new(8);
    let authored_hard = ArmorHardness::new(4);

    let mut app = headless_app();
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(cl(0, 0, 0), 0))
        .with_ganger(ganger_at(cl(1, 1, 0), 1))
        .wall_at(wall_cell)
        .build();
    app.world_mut().write_message(SetupBattleRequested::new(
        situation,
        crate::rng::BattleSeed::new(0xEF),
    ));
    app.update();

    // Look up the wall entity via the index.  Each `let Some` is preceded by a
    // runtime `assert!` so the else-return is a dead branch that only silences the
    // compiler; the test fails at the assert if the precondition is not met.
    let world = app.world_mut();
    let index_opt = world.get_resource::<TerrainIndex>();
    assert!(
        index_opt.is_some(),
        "Test 2: TerrainIndex must be present after setup"
    );
    let Some(index) = index_opt else {
        return;
    };
    let entity_opt = index.get(&TerrainIndexKey::Cover(wall_cell));
    assert!(
        entity_opt.is_some(),
        "Test 2: wall entity must be in TerrainIndex"
    );
    let Some(entity) = entity_opt else {
        return;
    };

    // Check all four stat components on the entity.
    let hp = world.get::<CoverHp>(entity).copied();
    let band = world.get::<HeightBand>(entity).copied();
    let prot = world.get::<ArmorProtection>(entity).copied();
    let hard = world.get::<ArmorHardness>(entity).copied();

    assert_eq!(
        hp,
        Some(authored_hp),
        "Test 2: wall entity must carry authored CoverHp (max)"
    );
    assert_eq!(
        band,
        Some(authored_band),
        "Test 2: wall entity must carry authored HeightBand"
    );
    assert_eq!(
        prot,
        Some(authored_prot),
        "Test 2: wall entity must carry authored ArmorProtection"
    );
    assert_eq!(
        hard,
        Some(authored_hard),
        "Test 2: wall entity must carry authored ArmorHardness"
    );
}

/// A slab entity carries [`SlabHp`], `ArmorProtection`, `ArmorHardness` from the
/// default slab prototype, and NO `HeightBand` (slab canon).
#[test]
fn test2_slab_entity_carries_prototype_stats_no_height_band() {
    let slab_cell = cl(3, 4, 1);
    let mut app = headless_app();
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(cl(0, 0, 0), 0))
        .with_ganger(ganger_at(cl(1, 1, 0), 1))
        .slab_at(slab_cell)
        .build();
    app.world_mut().write_message(SetupBattleRequested::new(
        situation,
        crate::rng::BattleSeed::new(0x01),
    ));
    app.update();

    let world = app.world_mut();
    let index_opt = world.get_resource::<TerrainIndex>();
    assert!(index_opt.is_some(), "Test 2: TerrainIndex must be present");
    let Some(index) = index_opt else {
        return;
    };
    let entity_opt = index.get(&TerrainIndexKey::Slab(slab_cell));
    assert!(
        entity_opt.is_some(),
        "Test 2: slab entity must be in TerrainIndex"
    );
    let Some(entity) = entity_opt else {
        return;
    };

    // `SlabHp` must be present.
    let hp = world.get::<SlabHp>(entity).copied();
    assert!(hp.is_some(), "Test 2: slab entity must carry SlabHp (max)");

    // `HeightBand` must NOT be present on a slab (slab canon — slabs span the whole
    // z-boundary, no band to clear).
    let band = world.get::<HeightBand>(entity);
    assert!(
        band.is_none(),
        "Test 2: slab entity must NOT carry HeightBand (slab canon — no band)",
    );

    // `ArmorProtection` and `ArmorHardness` from the prototype.
    assert!(
        world.get::<ArmorProtection>(entity).is_some(),
        "Test 2: slab entity must carry ArmorProtection",
    );
    assert!(
        world.get::<ArmorHardness>(entity).is_some(),
        "Test 2: slab entity must carry ArmorHardness",
    );
}
