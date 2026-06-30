//! GTW-395 acceptance tests — the per-tile terrain ECS entity layer.
//!
//! Each test corresponds to a named acceptance clause in the GTW-395 design:
//!
//! - **Test 1**: One entity per terrain piece spawned (C1).
//! - **Test 2**: Entity carries static stats from authored data (C2).
//! - **Test 3**: Cell-to-entity lookup via [`TerrainIndex`] (C4 / queryable-by-cell).
//! - **Test 5**: Index + ledger lifetime — both [`TerrainIndex`] AND [`SlabLedger`] are
//!   absent after teardown (blocker 1 / pre-existing leak fix).
//! - **Test 6**: Bridge pattern — [`TerrainIndex`] → entity → [`CoverHp`] (max); live HP
//!   from [`CoverLedger`] by the same key (C3).
//! - **Test 7**: Typed-key no-collision — a cover and a slab at the same [`CellLevel`]
//!   produce TWO distinct index entries (major-4 fix).

use bevy::{
    asset::AssetPlugin,
    ecs::message::Messages,
    prelude::{App, MinimalPlugins},
    scene::ScenePlugin,
};

use crate::{
    armor::{ArmorHardness, ArmorProtection},
    battle::{BattleReady, BattleSimPlugin, SetupBattleRequested, TeardownBattleRequested},
    cover::{CoverHp, CoverLedger, HeightBand},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    slab::{SlabHp, SlabLedger},
    terrain::entity::{
        BlocksPathfinding, TerrainCell, TerrainIndex, TerrainIndexKey, TerrainPieceKind,
    },
    test_support::{
        SituationBuilder, ganger_at, test_armor_registry, test_gang_registry,
        test_melee_weapon_registry, test_terrain_registry, test_weapon_registry,
    },
    tuning::{CombatTuning, GangerStatTuning},
};

// ── Helpers ──────────────────────────────────────────────────────────────────

/// A headless app with [`BattleSimPlugin`] — the same harness the battle-lifecycle
/// tests use ([`MinimalPlugins`] + [`AssetPlugin`] + [`ScenePlugin`] + [`BattleSimPlugin`]
/// + the persistent-Load-style resources).
fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning::default());
    app.insert_resource(GangerStatTuning::default());
    app.insert_resource(test_weapon_registry());
    // GTW-505: the melee registry (with the `fists` default) so each ganger's melee
    // weapon resolves at setup (fixture gangers author none -> `fists`).
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    // GTW-396: the terrain registry so SituationBuilder's wall_at / slab_at piece
    // keys ("test-wall", "test-slab") resolve at setup_battle_on_request.
    app.insert_resource(test_terrain_registry());
    // GTW-414/415: the GangRegistry the v2 setup_battle resolves each placed ganger's
    // (gang, member) ref against (the fixtures use `ganger_at`, whose members the canonical
    // `test_gang_registry` holds); without it setup fails closed and no ganger spawns.
    app.insert_resource(test_gang_registry());
    app
}

/// A helper `(cell, level)` at grid coordinates `(x, y, level)`.
fn cl(x: i32, y: i32, level: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(level))
}

/// Drain [`BattleReady`] messages and return the count.
fn drain_ready(app: &mut App) -> usize {
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .drain()
        .count()
}

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

// ── Test 3 — Queryable by cell (C4) ──────────────────────────────────────────

/// [`TerrainIndex`] resolves `Cover(cell)` and `Slab(cell)` to entities whose
/// [`TerrainCell`] component matches the resolved cell and [`TerrainPieceKind`] matches.
#[test]
fn test3_queryable_by_cell() {
    let wall_cell = cl(2, 2, 0);
    let slab_cell = cl(3, 4, 1);

    let mut app = headless_app();
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(cl(0, 0, 0), 0))
        .with_ganger(ganger_at(cl(1, 1, 0), 1))
        .wall_at(wall_cell)
        .slab_at(slab_cell)
        .build();
    app.world_mut().write_message(SetupBattleRequested::new(
        situation,
        crate::rng::BattleSeed::new(0x02),
    ));
    app.update();

    let world = app.world_mut();
    let index_opt = world.get_resource::<TerrainIndex>();
    assert!(index_opt.is_some(), "Test 3: TerrainIndex must be present");
    let Some(index) = index_opt else {
        return;
    };

    // Cover lookup.
    let cover_entity_opt = index.get(&TerrainIndexKey::Cover(wall_cell));
    assert!(
        cover_entity_opt.is_some(),
        "Test 3 (C4): wall must be queryable by cell"
    );
    let Some(cover_entity) = cover_entity_opt else {
        return;
    };
    let cover_cell_comp = world.get::<TerrainCell>(cover_entity).copied();
    let cover_kind = world.get::<TerrainPieceKind>(cover_entity).copied();
    assert_eq!(
        cover_cell_comp.as_deref().copied(),
        Some(wall_cell),
        "Test 3: TerrainCell must equal the authored wall cell",
    );
    assert_eq!(
        cover_kind,
        Some(TerrainPieceKind::Wall),
        "Test 3: wall entity kind must be Wall",
    );

    // Slab lookup.
    let slab_entity_opt = index.get(&TerrainIndexKey::Slab(slab_cell));
    assert!(
        slab_entity_opt.is_some(),
        "Test 3 (C4): slab must be queryable by cell"
    );
    let Some(slab_entity) = slab_entity_opt else {
        return;
    };
    let slab_cell_comp = world.get::<TerrainCell>(slab_entity).copied();
    let slab_kind = world.get::<TerrainPieceKind>(slab_entity).copied();
    assert_eq!(
        slab_cell_comp.as_deref().copied(),
        Some(slab_cell),
        "Test 3: TerrainCell must equal the authored slab cell",
    );
    assert_eq!(
        slab_kind,
        Some(TerrainPieceKind::Slab),
        "Test 3: slab entity kind must be Slab",
    );
}

// ── GTW-501 — spawn → project → is_path_blocked, end-to-end (C1/C3/C5) ───────

/// GTW-501 (C1 + C3 + C5, end-to-end on the REAL runtime) — after `setup_battle` spawns a
/// wall terrain entity and the `BattleSimPlugin` projection runs, the wall cell reads
/// `is_path_blocked` (the spawned wall entity carries the derived `BlocksPathfinding`
/// marker, and `project_path_blocking` folds it into the grid's path-blocking surface),
/// while an authored slab cell does NOT (a slab does not block the path by default). The
/// wall's KIND-based `is_blocked` (vision) is also still true — proving the path surface and
/// the vision surface AGREE for a kind-default wall (the C5 zero-regression guarantee).
#[test]
fn gtw501_wall_is_path_blocked_after_setup() {
    let wall_cell = cl(2, 2, 0);
    let slab_cell = cl(3, 4, 1);
    let open_cell = cl(5, 5, 0);

    let mut app = headless_app();
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(cl(0, 0, 0), 0))
        .with_ganger(ganger_at(cl(1, 1, 0), 1))
        .wall_at(wall_cell)
        .slab_at(slab_cell)
        .build();
    app.world_mut().write_message(SetupBattleRequested::new(
        situation,
        crate::rng::BattleSeed::new(0x07),
    ));
    // One tick: setup (`.before(Simulate)`) spawns the wall + inserts the grid + the
    // BattleInProgress witness; the Simulate band's `project_path_blocking` then folds the
    // Added<BlocksPathfinding> marker into the surface — all the same tick.
    app.update();

    let world = app.world_mut();

    // C1: the spawned wall entity carries the derived BlocksPathfinding marker.
    let mut marker_query = world.query::<(&TerrainCell, Option<&BlocksPathfinding>)>();
    let wall_has_marker = marker_query
        .iter(world)
        .find(|(cell, _)| ***cell == wall_cell)
        .map(|(_, m)| m.is_some());
    assert_eq!(
        wall_has_marker,
        Some(true),
        "C1: the spawned wall entity must carry the derived BlocksPathfinding marker",
    );

    // C3/C5: the projection folded it into the grid's path-blocking surface.
    let grid = world.get_resource::<OccupancyGrid>();
    assert!(grid.is_some(), "setup must insert the OccupancyGrid");
    let Some(grid) = grid else {
        return;
    };
    assert!(
        grid.is_path_blocked(&wall_cell),
        "C3/C5: the wall cell must be PATH-blocked after the projection runs",
    );
    assert!(
        grid.is_blocked(&wall_cell),
        "C5: the wall cell is ALSO kind-blocked (vision) — path & vision agree for a wall",
    );
    assert!(
        !grid.is_path_blocked(&slab_cell),
        "C1: a slab does NOT block the path by default (no marker → not in the surface)",
    );
    assert!(
        !grid.is_path_blocked(&open_cell),
        "an empty cell is not path-blocked",
    );
}

// ── Test 5 — Index + ledger lifetime (blocker 1 / SlabLedger leak fix) ───────

/// After setup, [`TerrainIndex`] and [`SlabLedger`] are present.
/// After teardown, **both** are absent — confirming the pre-existing [`SlabLedger`]
/// leak is fixed alongside [`TerrainIndex`]'s own teardown.
#[test]
fn test5_terrain_index_and_slab_ledger_removed_on_teardown() {
    let mut app = headless_app();

    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(cl(0, 0, 0), 0))
        .with_ganger(ganger_at(cl(1, 1, 0), 1))
        .wall_at(cl(2, 2, 0))
        .slab_at(cl(3, 3, 1))
        .build();

    // Setup.
    app.world_mut().write_message(SetupBattleRequested::new(
        situation,
        crate::rng::BattleSeed::new(0x03),
    ));
    app.update();
    drain_ready(&mut app);

    // Precondition: both present after setup.
    assert!(
        app.world().get_resource::<TerrainIndex>().is_some(),
        "Test 5 precondition: TerrainIndex must be present after setup",
    );
    assert!(
        app.world().get_resource::<SlabLedger>().is_some(),
        "Test 5 precondition: SlabLedger must be present after setup",
    );

    // Teardown.
    app.world_mut().write_message(TeardownBattleRequested);
    app.update();

    // Both absent after teardown.
    assert!(
        app.world().get_resource::<TerrainIndex>().is_none(),
        "Test 5 (blocker 1): TerrainIndex must be absent after teardown",
    );
    assert!(
        app.world().get_resource::<SlabLedger>().is_none(),
        "Test 5 (blocker 1, leak fix): SlabLedger must be absent after teardown (pre-existing \
         leak — it was never removed until GTW-395)",
    );
}

// ── Test 6 — Bridge pattern (C3) ─────────────────────────────────────────────

/// [`TerrainIndex`] → entity → static `CoverHp` (max from component);
/// live HP from [`CoverLedger`] by the same key.
/// After setup, both the entity's `CoverHp` and the ledger's live HP are at max
/// (proving HP authority lives in the ledger, but the entity is the static ceiling).
#[test]
fn test6_bridge_max_hp_on_entity_live_hp_in_ledger() {
    let wall_cell = cl(2, 2, 0);
    let authored_max = CoverHp::new(120);

    let mut app = headless_app();
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(cl(0, 0, 0), 0))
        .with_ganger(ganger_at(cl(1, 1, 0), 1))
        .wall_at(wall_cell)
        .build();
    app.world_mut().write_message(SetupBattleRequested::new(
        situation,
        crate::rng::BattleSeed::new(0x04),
    ));
    app.update();

    let world = app.world_mut();

    // Look up the entity via TerrainIndex.
    let index_opt = world.get_resource::<TerrainIndex>();
    assert!(index_opt.is_some(), "Test 6: TerrainIndex must be present");
    let Some(index) = index_opt else {
        return;
    };
    let entity_opt = index.get(&TerrainIndexKey::Cover(wall_cell));
    assert!(entity_opt.is_some(), "Test 6: wall must be in TerrainIndex");
    let Some(entity) = entity_opt else {
        return;
    };

    // The entity's CoverHp is the static max.
    let entity_max = world.get::<CoverHp>(entity).copied();
    assert_eq!(
        entity_max,
        Some(authored_max),
        "Test 6 (C3): entity CoverHp must equal authored max",
    );

    // Peek at the ledger — the wall was seeded by setup, so it has an entry at max.
    let ledger_opt = world.get_resource::<CoverLedger>();
    assert!(ledger_opt.is_some(), "Test 6: CoverLedger must be present");
    let Some(ledger) = ledger_opt else {
        return;
    };
    let ledger_entry = ledger.peek(&wall_cell);
    assert!(
        ledger_entry.is_some(),
        "Test 6: CoverLedger must have an entry for the authored wall cell after setup",
    );
    let live_hp = ledger_entry.map(|e| e.current_hp);
    assert_eq!(
        live_hp,
        Some(authored_max),
        "Test 6 (bridge): live HP from CoverLedger must equal max at setup (no hits yet)",
    );
}

// ── Test 7 — Typed-key no-collision (major 4) ────────────────────────────────

/// A cover and a slab at the **same `CellLevel`** produce TWO distinct entries in
/// [`TerrainIndex`] (keyed `Cover(cell)` and `Slab(cell)` respectively) — neither
/// silently overwrites the other.
#[test]
fn test7_typed_key_no_collision_at_shared_cell() {
    // Place a wall AND a slab at the SAME cell (level 0).
    let shared_cell = cl(5, 5, 0);

    let mut app = headless_app();
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(cl(0, 0, 0), 0))
        .with_ganger(ganger_at(cl(1, 1, 0), 1))
        .wall_at(shared_cell)
        .slab_at(shared_cell)
        .build();
    app.world_mut().write_message(SetupBattleRequested::new(
        situation,
        crate::rng::BattleSeed::new(0x05),
    ));
    app.update();

    let world = app.world_mut();
    let index_opt = world.get_resource::<TerrainIndex>();
    assert!(index_opt.is_some(), "Test 7: TerrainIndex must be present");
    let Some(index) = index_opt else {
        return;
    };

    // The index must have at least 2 entries (wall + slab) — no silent overwrite.
    assert!(
        index.len() >= 2,
        "Test 7 (major 4): TerrainIndex must have at least 2 entries for a shared-cell \
         wall + slab (Cover(cell) and Slab(cell) are distinct keys)",
    );

    // Both entries are independently resolvable.
    let cover_entity = index.get(&TerrainIndexKey::Cover(shared_cell));
    let slab_entity = index.get(&TerrainIndexKey::Slab(shared_cell));
    assert!(
        cover_entity.is_some(),
        "Test 7: Cover(shared_cell) must resolve to an entity",
    );
    assert!(
        slab_entity.is_some(),
        "Test 7: Slab(shared_cell) must resolve to an entity",
    );

    // They must be DISTINCT entities.
    assert_ne!(
        cover_entity, slab_entity,
        "Test 7: the cover and slab entities at the shared cell must be distinct",
    );
}
