//! HARNESS NOTE: this file builds the `App` itself — `MinimalPlugins`, `AssetPlugin`,
//! `ScenePlugin` and `BattleSimPlugin`.
use bevy::{app::App, prelude::MinimalPlugins, scene::ScenePlugin};
use cobalt_test_utils::unwatched_asset_plugin;
use gdtf_battle_sim::{
    battle::{BattleSimPlugin, SetupBattleRequested},
    cover::HeightBand,
    entity::BlocksVision,
    ganger::GangRegistry,
    metric::{Cell, CellLevel, Level},
    prelude::{Faction, Stance, StanceKind},
    rng::BattleSeed,
    situation::{CoverSpawn, PlacedGanger, Situation},
    terrain::{entity::TerrainCell, facing::TerrainFacing, occupancy::OccupancyGrid},
    test_support::{
        GangerSpawnBuilder, SituationBuilder, test_armor_registry, test_melee_weapon_registry,
        test_pieces, test_terrain_registry, test_weapon_registry,
    },
    tuning::{CombatTuning, ViewRange},
    visibility::SquadVisibility,
};

const SEED: u64 = 0x5025_0202;

const PLAYER: u8 = 0;

const TEST_VIEW_RANGE: u16 = 10;

fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

fn observer_at() -> CellLevel {
    ground(3, 5)
}
fn target_at() -> CellLevel {
    ground(7, 5)
}
fn occluder_at() -> CellLevel {
    ground(5, 5)
}

fn battle_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, unwatched_asset_plugin(), ScenePlugin));
    app.add_plugins(BattleSimPlugin);
    app.insert_resource(CombatTuning {
        view_range: ViewRange::new(TEST_VIEW_RANGE),
        ..Default::default()
    });
    app.insert_resource(test_weapon_registry());
    app.insert_resource(test_melee_weapon_registry());
    app.insert_resource(test_armor_registry());
    app.insert_resource(test_terrain_registry());
    app
}

fn drive_setup(app: &mut App, situation_and_gangs: (Situation, Vec<PlacedGanger>, GangRegistry)) {
    let (situation, placements, gangs) = situation_and_gangs;
    app.world_mut().insert_resource(gangs);
    app.world_mut().write_message(SetupBattleRequested::new(
        situation,
        placements,
        BattleSeed::new(SEED),
    ));
    app.update();
    app.update();
    app.update();
}

fn squad(app: &App) -> Option<SquadVisibility> {
    app.world().get_resource::<SquadVisibility>().cloned()
}

fn observer_situation(
    add_terrain: impl FnOnce(SituationBuilder) -> SituationBuilder,
) -> (Situation, Vec<PlacedGanger>, GangRegistry) {
    let builder = SituationBuilder::new().with_ganger(
        GangerSpawnBuilder::new()
            .at(observer_at())
            .faction(Faction::new(PLAYER))
            .stance(Stance::new(StanceKind::Standing))
            .build(),
    );
    add_terrain(builder).build_with_gangs()
}

#[test]
fn tagged_slab_occludes_but_untagged_slab_does_not() {
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

#[test]
fn path_only_slab_does_not_occlude_vision() {
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

    assert!(
        grid.vision_occluder_at(&vision_only).is_some(),
        "the BlocksVision-tagged slab occludes vision (it is on the vision surface)",
    );
    assert!(
        !*grid.is_path_blocked(&vision_only),
        "C8d: a vision-only occluder does NOT block a path (it is NOT on the path surface) — \
         the two surfaces are independent (C7)",
    );

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

#[test]
fn occluder_band_gates_the_sightline() {
    let mut low_app = battle_app();
    drive_setup(
        &mut low_app,
        observer_situation(|b| {
            b.with_scatter(CoverSpawn::new(
                occluder_at(),
                test_pieces::LOW_VISION_COVER,
                TerrainFacing::default(),
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

#[test]
fn adding_and_removing_occluder_flips_visibility() {
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

fn find_terrain_entity(app: &mut App, at: CellLevel) -> Option<bevy::prelude::Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(bevy::prelude::Entity, &TerrainCell)>();
    query
        .iter(world)
        .find(|(_, cell)| ***cell == at)
        .map(|(entity, _)| entity)
}
