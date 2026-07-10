//! GTW-502 (child 482b) — the tag-driven, HEIGHT-AWARE vision occluder, driven end-to-end on
//! the REAL LoS/FoV path through the live battle runtime.
//!
//! Proves the occluder behaves per the clause contract by driving the SAME
//! `setup_battle_on_request` → `BattleSimPlugin` `Simulate`-band path the GTW-341 squad-fog
//! tests use, then asserting on the precomputed [`SquadVisibility`]: a cell BEHIND an occluder
//! drops out of the squad FOV iff the occluder occludes at the relevant band.
//!
//! Clauses exercised here (the in-engine, real-path half of GTW-502 C8):
//!
//! - **C8a/C8b (the discriminating pair)** — a `Slab` EXPLICITLY tagged `BlocksVision`
//!   occludes the sightline (the cell behind it is NOT in the FOV), while the SAME `Slab`
//!   WITHOUT the tag does NOT occlude (the cell behind it IS in the FOV). The two situations
//!   differ ONLY by the tag, so the tag is the cause.
//! - **C8c (zero regression)** — an existing `Wall` still occludes the sightline exactly as
//!   before (the cell behind it is NOT in the FOV).
//! - **C8d (independence)** — a `Slab` tagged ONLY `BlocksPathfinding` (not `BlocksVision`)
//!   does NOT occlude vision (the cell behind it IS in the FOV); the vision-only Wall does NOT
//!   block a path (the pathfinder still finds a route past it). Vision and path are
//!   orthogonal (C7).
//! - **C8e (height-awareness)** — a LOW-band `Cover` occluder occludes a standing observer's
//!   LOW eye-line to a prone target (a Low aim) but is cleared by a HIGH eye-line to a
//!   standing target — the band gate, asserted as a sighted/unsighted flip with the SAME
//!   occluder.
//! - **C8f (change-detection)** — adding / removing the `BlocksVision` component at runtime
//!   flips the squad FOV through the REAL recompute (the cell behind the occluder leaves /
//!   re-enters VISIBLE).
//!
//! HARNESS NOTE: the sim is the LOW crate (a dev-dep on `gdtf_test_utils` would be a cycle),
//! so this uses the established sim-crate battle-integration idiom — drive
//! `setup_battle_on_request` via `SetupBattleRequested` against a `MinimalPlugins` +
//! `AssetPlugin` + `ScenePlugin` + `BattleSimPlugin` app, `app.update()`-driven, `world_mut()`
//! for setup / mutation / assertion (bevy-traps #7 headless-test carve-out) — the same harness
//! as `squad_fog_recompute.rs`.

use bevy::{app::App, asset::AssetPlugin, prelude::MinimalPlugins, scene::ScenePlugin};
use gdtf_battle_sim::{
    battle::{BattleSimPlugin, SetupBattleRequested},
    cover::HeightBand,
    entity::BlocksVision,
    ganger::GangRegistry,
    metric::{Cell, CellLevel, Level},
    prelude::{Faction, Stance, StanceKind},
    rng::BattleSeed,
    situation::{CoverSpawn, Situation},
    terrain::{entity::TerrainCell, occupancy::OccupancyGrid},
    test_support::{
        GangerSpawnBuilder, SituationBuilder, test_armor_registry, test_melee_weapon_registry,
        test_pieces, test_terrain_registry, test_weapon_registry,
    },
    tuning::{CombatTuning, ViewRange},
    visibility::SquadVisibility,
};

/// An arbitrary (not shipped tuning) seed for the test battle's RNG stream.
const SEED: u64 = 0x5025_0202;

/// Gang 0 is the player.
const PLAYER: u8 = 0;

/// A view range that comfortably spans the observer→target gap in every fixture below so the
/// sightline (not the range disc) is what decides visibility. Arbitrary test tuning.
const TEST_VIEW_RANGE: u16 = 10;

/// A ground-floor `(cell, level)` key.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// The observer's cell (West) and the target cell (East), on one clear row with the occluder
/// strictly between them at `(5, 5, 0)`.
fn observer_at() -> CellLevel {
    ground(3, 5)
}
fn target_at() -> CellLevel {
    ground(7, 5)
}
fn occluder_at() -> CellLevel {
    ground(5, 5)
}

/// Build the FULL live-runtime harness (the GTW-341 precedent): `MinimalPlugins` +
/// `AssetPlugin` + `ScenePlugin` + `BattleSimPlugin`, with a long view range + the test
/// registries seeded.
fn battle_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(test_weapon_registry());
    // GTW-505: the melee registry (with the `fists` default) so each ganger's melee
    // weapon resolves at setup (fixture gangers author none -> `fists`).
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    // GTW-491: setup_battle_on_request resolves authored terrain UUIDs (walls / slabs /
    // scatter) against this registry — without it the cover/slab pieces fail TerrainNotFound
    // and setup aborts (no SquadVisibility inserted).
    app.insert_resource(test_terrain_registry());
    app
}

/// Drive a setup through the REAL `setup_battle_on_request` Ok path and settle it (the GTW-341
/// `drive_setup`): the deferred `bsn!` ganger scenes materialize and the
/// `Changed<Position>`-triggered recompute fills the fog.
fn drive_setup(app: &mut App, situation_and_gangs: (Situation, GangRegistry)) {
    let (situation, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .write_message(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    app.update();
    app.update();
    app.update();
}

/// Read the current `SquadVisibility` snapshot (cloned), if present.
fn squad(app: &App) -> Option<SquadVisibility> {
    app.world().get_resource::<SquadVisibility>().cloned()
}

/// A ONE-player situation: a standing player ganger at the observer cell, looking East along a
/// clear row, with whatever terrain the caller threads in via `add_terrain`.
fn observer_situation(
    add_terrain: impl FnOnce(SituationBuilder) -> SituationBuilder,
) -> (Situation, GangRegistry) {
    let builder = SituationBuilder::new().with_ganger(
        GangerSpawnBuilder::new()
            .at(observer_at())
            .faction(Faction::new(PLAYER))
            .stance(Stance::new(StanceKind::Standing))
            .build(),
    );
    add_terrain(builder).build_with_gangs()
}

// === C8a / C8b — the discriminating pair: a Slab occludes the sightline iff it is tagged
// BlocksVision. ===

#[test]
fn tagged_slab_occludes_but_untagged_slab_does_not() {
    // (a) The TAGGED slab (VISION_SLAB carries BlocksVision) occludes: the cell behind it is
    //     NOT visible.
    let mut tagged = battle_app();
    drive_setup(
        &mut tagged,
        observer_situation(|b| b.slab_piece_at(occluder_at(), test_pieces::VISION_SLAB)),
    );
    let Some(fog_tagged) = squad(&tagged) else {
        unreachable!("setup inserts SquadVisibility");
    };
    assert!(
        *fog_tagged.is_cell_visible(&observer_at()),
        "sanity: the observer always sees its own cell",
    );
    assert!(
        !*fog_tagged.is_cell_visible(&target_at()),
        "C8a: a Slab tagged BlocksVision OCCLUDES — the cell behind it is NOT in the FOV",
    );

    // (b) The SAME slab WITHOUT the tag (plain SLAB) does NOT occlude: the cell behind it IS
    //     visible. The ONLY difference between the two situations is the tag.
    let mut untagged = battle_app();
    drive_setup(
        &mut untagged,
        observer_situation(|b| b.slab_piece_at(occluder_at(), test_pieces::SLAB)),
    );
    let Some(fog_untagged) = squad(&untagged) else {
        unreachable!("setup inserts SquadVisibility");
    };
    assert!(
        *fog_untagged.is_cell_visible(&target_at()),
        "C8b: the SAME Slab WITHOUT the tag does NOT occlude — the cell behind it IS in the FOV \
         (the discriminating pair: only the BlocksVision tag changed the verdict)",
    );
}

// === C8c — zero regression: an existing Wall still occludes the sightline. ===

#[test]
fn existing_wall_still_occludes() {
    let mut app = battle_app();
    drive_setup(&mut app, observer_situation(|b| b.wall_at(occluder_at())));
    let Some(fog) = squad(&app) else {
        unreachable!("setup inserts SquadVisibility");
    };
    assert!(
        *fog.is_cell_visible(&observer_at()),
        "sanity: the observer sees its own cell",
    );
    assert!(
        !*fog.is_cell_visible(&target_at()),
        "C8c (zero regression): an existing Wall still OCCLUDES the sightline (cell behind it \
         is NOT in the FOV) — the tag-derived occluder reproduces the wall's existing behaviour",
    );
}

// === C8d — independence: path-only does not occlude vision; vision-only does not block path. ===

#[test]
fn path_only_slab_does_not_occlude_vision() {
    // A Slab tagged ONLY BlocksPathfinding (PATH_SLAB) blocks a path but must NOT occlude
    // vision: the cell behind it IS visible.
    let mut app = battle_app();
    drive_setup(
        &mut app,
        observer_situation(|b| b.slab_piece_at(occluder_at(), test_pieces::PATH_SLAB)),
    );
    let Some(fog) = squad(&app) else {
        unreachable!("setup inserts SquadVisibility");
    };
    assert!(
        *fog.is_cell_visible(&target_at()),
        "C8d: a Slab tagged ONLY BlocksPathfinding does NOT occlude vision (cell behind it IS \
         visible) — vision and path are independent (C7)",
    );
}

#[test]
fn vision_and_path_surfaces_are_independent_on_the_grid() {
    // The two surfaces are orthogonal (C7), asserted DIRECTLY on the live OccupancyGrid the
    // sim builds — the authoritative split GTW-501 + GTW-502 introduced. Author BOTH a
    // vision-only occluder (a BlocksVision-tagged slab, NO BlocksPathfinding) AND a path-only
    // blocker (a BlocksPathfinding-tagged slab, NO BlocksVision) on distinct cells in one
    // battle, then read each cell on each surface.
    let vision_only = ground(2, 2);
    let path_only = ground(8, 8);
    let mut app = battle_app();
    drive_setup(
        &mut app,
        observer_situation(|b| {
            b.slab_piece_at(vision_only, test_pieces::VISION_SLAB)
                .slab_piece_at(path_only, test_pieces::PATH_SLAB)
        }),
    );

    let Some(grid) = app.world().get_resource::<OccupancyGrid>().cloned() else {
        unreachable!("setup inserts the OccupancyGrid");
    };

    // The vision-only slab OCCLUDES vision but does NOT block a path.
    assert!(
        grid.vision_occluder_at(&vision_only).is_some(),
        "the BlocksVision-tagged slab occludes vision (it is on the vision surface)",
    );
    assert!(
        !*grid.is_path_blocked(&vision_only),
        "C8d: a vision-only occluder does NOT block a path (it is NOT on the path surface) — \
         the two surfaces are independent (C7)",
    );

    // The path-only slab blocks a path but does NOT occlude vision.
    assert!(
        *grid.is_path_blocked(&path_only),
        "the BlocksPathfinding-tagged slab blocks a path (it is on the path surface)",
    );
    assert!(
        grid.vision_occluder_at(&path_only).is_none(),
        "C8d: a path-only blocker does NOT occlude vision (it is NOT on the vision surface) — \
         the two surfaces are independent (C7)",
    );
}

// === C8e — height-awareness: a LOW occluder blocks a LOW eye-line but a HIGH one clears it. ===

#[test]
fn occluder_band_gates_the_sightline() {
    // The band GATE, asserted as a discriminating pair on the OCCLUDER's band with a fixed
    // STANDING observer (whose eye-line rises above the LOW band over the gap but not above the
    // HIGH band): a LOW-band Cover occluder is CLEARED (target visible) while a HIGH-band
    // occluder (a Wall) OCCLUDES (target not visible). Same observer, same geometry — only the
    // occluder's band differs, isolating the height-aware `round_clears_occupant` gate.

    // (low) — a LOW-band Cover occluder; the standing eye-line clears it.
    let mut low_app = battle_app();
    drive_setup(
        &mut low_app,
        observer_situation(|b| {
            b.with_scatter(CoverSpawn::new(
                occluder_at(),
                test_pieces::LOW_VISION_COVER,
            ))
        }),
    );
    let Some(low_fog) = squad(&low_app) else {
        unreachable!("setup inserts SquadVisibility");
    };
    assert!(
        *low_fog.is_cell_visible(&target_at()),
        "C8e: a LOW-band occluder is CLEARED by the standing observer's higher eye-line (target \
         IS visible) — a sightline strictly above the occluder's band sails over it",
    );

    // (high) — a HIGH-band occluder (a Wall) at the SAME cell; the SAME eye-line is occluded.
    let mut high_app = battle_app();
    drive_setup(
        &mut high_app,
        observer_situation(|b| b.wall_at(occluder_at())),
    );
    let Some(high_fog) = squad(&high_app) else {
        unreachable!("setup inserts SquadVisibility");
    };
    assert!(
        !*high_fog.is_cell_visible(&target_at()),
        "C8e: a HIGH-band occluder at the SAME cell OCCLUDES the SAME eye-line (target NOT \
         visible) — the band gate flips the verdict with only the occluder band changed",
    );
}

// === C8f — change-detection: adding / removing the BlocksVision component flips the FOV. ===

#[test]
fn adding_and_removing_occluder_flips_visibility() {
    // Start with the plain (untagged) slab so the target is visible, then RUNTIME-ADD a
    // BlocksVision component to the slab entity → the cell behind it leaves VISIBLE; then
    // REMOVE it → the cell re-enters VISIBLE. Drives the REAL recompute through the C6 trigger.
    let mut app = battle_app();
    drive_setup(
        &mut app,
        observer_situation(|b| b.slab_piece_at(occluder_at(), test_pieces::SLAB)),
    );

    let Some(before) = squad(&app) else {
        unreachable!("setup inserts SquadVisibility");
    };
    assert!(
        *before.is_cell_visible(&target_at()),
        "precondition: with the untagged slab the cell behind it is VISIBLE",
    );

    // RUNTIME-ADD the occluder component to the slab entity (find it by its TerrainCell).
    let Some(slab_entity) = find_terrain_entity(&mut app, occluder_at()) else {
        unreachable!("a terrain entity was authored at the occluder cell");
    };
    app.world_mut()
        .entity_mut(slab_entity)
        .insert(BlocksVision::new(HeightBand::High));
    app.update();
    app.update();

    let Some(occluded) = squad(&app) else {
        unreachable!("SquadVisibility persists");
    };
    assert!(
        !*occluded.is_cell_visible(&target_at()),
        "C8f: ADDING BlocksVision at runtime occludes the sightline (the cell behind it leaves \
         VISIBLE) — the real recompute fired off the C6 Added<BlocksVision> trigger",
    );

    // RUNTIME-REMOVE it → the sightline re-opens.
    app.world_mut()
        .entity_mut(slab_entity)
        .remove::<BlocksVision>();
    app.update();
    app.update();

    let Some(reopened) = squad(&app) else {
        unreachable!("SquadVisibility persists");
    };
    assert!(
        *reopened.is_cell_visible(&target_at()),
        "C8f: REMOVING BlocksVision re-opens the sightline (the cell behind it re-enters \
         VISIBLE) — the real recompute fired off the C6 RemovedComponents<BlocksVision> trigger",
    );
}

/// Find the terrain entity occupying `at` by scanning the `TerrainCell` components — a
/// `world_mut()` query fixture probe (bevy-traps #7 headless-test carve-out). Returns `None`
/// if absent (kept `Option` so the test never `expect`s; the restriction lints fire in tests
/// too — the caller resolves it with a `let ... else { unreachable!() }`).
fn find_terrain_entity(app: &mut App, at: CellLevel) -> Option<bevy::prelude::Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(bevy::prelude::Entity, &TerrainCell)>();
    query
        .iter(world)
        .find(|(_, cell)| ***cell == at)
        .map(|(entity, _)| entity)
}
