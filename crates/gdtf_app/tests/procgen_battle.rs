//! GTW-433 (situation procgen migration): a theme+size-only situation procgen-generates
//! its terrain at `BattleScapeState::Generation` and reaches
//! `BattleScapeState::BattleRunning` with a PLAYABLE (terrain-populated) level.
//!
//! This is the END-TO-END proof that the procgen trigger is LIVE: the authored situation
//! carries only `theme` + `grid_size` + gangers (no inline terrain), and the app-side
//! Generation system drives the sim's `generate_level` (GTW-431) against the loaded prefab
//! library to fill in the terrain before the battle is built.
//!
//! Two pin-discriminating tests:
//!
//! 1. [`skirmish_ron_authors_no_terrain`] — the SHIPPED `assets/situations/skirmish.ron`
//!    authors theme + `grid_size` + gangers and NO inline terrain (C1: the migration removed
//!    walls / scatter / slabs / `default_floor` / floors / `vertical_links`).
//! 2. [`procgen_battle_reaches_running_with_populated_terrain`] — the full app walk drives a
//!    theme+size-only situation through Generation, REACHES `BattleRunning`, and the battle's
//!    `TerrainIndex` is NON-EMPTY — terrain the authored situation did NOT contain, so it can
//!    only have come from the live procgen path (C2/C3).
//!
//! Headless `MinimalPlugins` (via [`GdtfTestAppBuilder`]) — they bypass the `Load` scene, so
//! each injects the persistent `Load` resources it relies on, including the REAL
//! [`PrefabRegistry`] (built from the shipped `assets/content/maps/industrial_hive/12x12/*`
//! player + enemy prefabs) and a [`TerrainRegistry`] holding the pieces those prefabs
//! reference, so `generate_level` assembles a real level the setup can resolve.

use bevy::state::state::State;
use gdtf_app::test_support::{BattleScapeState, LoadedSituation, RunningState};
use gdtf_battle_sim::{
    injuries::InjuryRegistry,
    level::{
        GridHeight, GridLevels, GridSize, GridWidth, LevelTheme, Prefab, PrefabName,
        PrefabRegistry, PrefabSpec, ThemeCatalogRegistry,
    },
    metric::{Cell, CellLevel, Level},
    rng::BattleSeed,
    situation::Situation,
    terrain::{
        entity::TerrainIndex,
        piece::{TerrainName, TerrainRegistry, TerrainSpec},
    },
    test_support::{ganger_at, test_armor_registry, test_weapon_registry},
    tuning::CombatTuning,
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// A budget large enough to drive the deep walk down into the battlescape, but bounded so a
/// machine that never reaches the predicate fails instead of hanging.
const BUDGET: u32 = 96;

/// A fixed, INJECTED per-battle seed (GTW-433 C2) — pinned so the procgen level is
/// reproducible across runs (no `thread_rng` / wall-clock). Pre-inserted as a `BattleSeed`
/// resource so `request_battle_setup` uses it rather than `resolve_root_seed`.
const TEST_SEED: u64 = 0x600D_5EED;

/// A `(cell, level)` on level 0.
fn at(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// The 30x30x4 skirmish board `GridSize` (the migrated `skirmish.ron` extent). Falls back to
/// the default extent if the explicit one fails to validate (it cannot — 30/30/4 are all
/// under the maxima — but the no-panic contract is honoured).
fn skirmish_board() -> GridSize {
    GridSize::new(GridWidth::new(30), GridHeight::new(30), GridLevels::new(4)).unwrap_or_default()
}

/// Build the REAL `PrefabRegistry` from the two shipped `IndustrialHive` 12x12 deployment
/// prefabs (the player + enemy fragments GTW-433 authored), so `generate_level` has a real
/// player- and enemy-role prefab to assemble a level from.
///
/// Deserialises each shipped `.prefab.ron` via `include_str!` (the same RON the asset loader
/// reads) and inserts it through the real [`Prefab::new`] validation + [`PrefabRegistry`]
/// path. A parse / validation failure yields an EMPTY registry (the no-panic contract); the
/// walk would then fall back to the authored terrain and the populated-terrain assertion
/// would fail loudly, surfacing the bad content rather than hanging.
fn real_prefab_registry() -> PrefabRegistry {
    let mut registry = PrefabRegistry::default();
    let prefabs = [
        (
            "player_deployment",
            include_str!(
                "../../../assets/content/maps/industrial_hive/12x12/player_deployment.prefab.ron"
            ),
        ),
        (
            "enemy_deployment",
            include_str!(
                "../../../assets/content/maps/industrial_hive/12x12/enemy_deployment.prefab.ron"
            ),
        ),
    ];
    for (stem, ron) in prefabs {
        let Ok(spec) = ron::from_str::<PrefabSpec>(ron) else {
            continue;
        };
        if let Ok(prefab) = Prefab::new(PrefabName::new(stem.to_owned()), spec) {
            registry.insert(prefab);
        }
    }
    registry
}

/// A `TerrainRegistry` holding the three pieces the GTW-433 deployment prefabs reference
/// (`deck_floor` / `bulkhead_wall` / `barricade`), deserialised from the SHIPPED
/// `assets/content/terrain/*.terrain.ron` so the procgen-generated terrain resolves at setup
/// (the real app builds the same registry from the terrain folder).
///
/// A piece that fails to parse is skipped (no-panic); a missing piece would make
/// `setup_battle` fail closed with `TerrainNotFound`, so the walk would not reach
/// `BattleRunning` and the test would fail loudly.
fn prefab_terrain_registry() -> TerrainRegistry {
    let mut registry = TerrainRegistry::default();
    let pieces = [
        (
            "deck_floor",
            include_str!("../../../assets/content/terrain/deck_floor.terrain.ron"),
        ),
        (
            "bulkhead_wall",
            include_str!("../../../assets/content/terrain/bulkhead_wall.terrain.ron"),
        ),
        (
            "barricade",
            include_str!("../../../assets/content/terrain/barricade.terrain.ron"),
        ),
    ];
    for (name, ron) in pieces {
        if let Ok(spec) = ron::from_str::<TerrainSpec>(ron) {
            registry.insert(TerrainName::new(name.to_owned()), spec);
        }
    }
    registry
}

/// A theme+size-only [`Situation`] mirroring the migrated `skirmish.ron`: `IndustrialHive`,
/// the 30x30x4 board, two gangers (factions 0/1), and ZERO inline terrain. Returns it with a
/// matching [`GangRegistry`] so the placed gangers resolve at setup.
fn theme_size_only_situation() -> (Situation, gdtf_battle_sim::ganger::GangRegistry) {
    let (mut situation, gangs) = gdtf_battle_sim::test_support::SituationBuilder::new()
        .with_gangers([ganger_at(at(5, 6), 0), ganger_at(at(12, 9), 1)])
        .build_with_gangs();
    // Mirror the migrated skirmish.ron: explicit theme + 30x30x4 board, no inline terrain
    // (the builder leaves walls/scatter/slabs/floors/links empty). Procgen fills the terrain.
    situation.theme = LevelTheme::IndustrialHive;
    situation.grid_size = skirmish_board();
    (situation, gangs)
}

/// Reads the current [`BattleScapeState`] if it is active.
fn battlescape_state(app: &bevy::app::App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// Reads the current [`RunningState`] if it is active.
fn running_state(app: &bevy::app::App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Stands in for the player at the menu (it no longer auto-advances, GTW-121): advances
/// until [`RunningState::Menu`] rests, then queues `Menu → Options`.
fn drive_past_menu(app: &mut bevy::app::App) -> bool {
    let reached = advance_until(
        app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    if reached {
        app.world_mut()
            .resource_mut::<bevy::state::state::NextState<RunningState>>()
            .set(RunningState::Options);
    }
    reached
}

/// Builds the headless walk app for the procgen path: a theme+size-only `LoadedSituation`
/// plus the REAL prefab + terrain registries (so `generate_level` assembles a real level the
/// setup resolves) and a FIXED injected `BattleSeed` (so procgen is reproducible).
fn procgen_walk_app(
    situation: Situation,
    gangs: gdtf_battle_sim::ganger::GangRegistry,
) -> bevy::app::App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .default_start()
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(test_weapon_registry());
    app.world_mut().insert_resource(test_armor_registry());
    // The TERRAIN registry holding the pieces the procgen prefabs reference (so the
    // generated walls/scatter/floor resolve at setup) — NOT the empty default.
    app.world_mut().insert_resource(prefab_terrain_registry());
    app.world_mut()
        .insert_resource(ThemeCatalogRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    // The gang registry the theme+size situation's placed gangers resolve against.
    app.world_mut().insert_resource(gangs);
    // The REAL prefab library — the player + enemy IndustrialHive deployment fragments — so
    // `generate_level` can assemble a level (an empty registry would fail closed and fall
    // back to the authored empty terrain).
    app.world_mut().insert_resource(real_prefab_registry());
    // The theme+size-only authored situation (no inline terrain) the Generation setup
    // procgen-fills.
    app.world_mut()
        .insert_resource(LoadedSituation::new(situation));
    // GTW-433 C2: a FIXED injected per-battle seed so procgen is deterministic / reproducible
    // (request_battle_setup prefers this over the wall-clock resolve_root_seed).
    app.world_mut().insert_resource(BattleSeed::new(TEST_SEED));
    app
}

/// C1: the SHIPPED `skirmish.ron` authors theme + `grid_size` + gangers and NO inline terrain.
///
/// Deserialises the real asset file (the same RON the loader reads) and asserts every terrain
/// list is empty and `default_floor` is unset, while theme / `grid_size` / gangers are present.
/// Pin: re-authoring any terrain entry into `skirmish.ron` (un-migrating it) turns this red.
#[test]
fn skirmish_ron_authors_no_terrain() {
    let ron = include_str!("../../../assets/situations/skirmish.ron");
    let parsed = ron::from_str::<Situation>(ron);
    assert!(
        parsed.is_ok(),
        "the shipped skirmish.ron must deserialize as a Situation: {:?}",
        parsed.as_ref().err(),
    );
    let Ok(situation) = parsed else {
        return;
    };

    // Theme + size + gangers are authored (the kept fields).
    assert_eq!(
        situation.theme,
        LevelTheme::IndustrialHive,
        "skirmish.ron must author its theme (IndustrialHive)",
    );
    assert_eq!(
        situation.grid_size,
        skirmish_board(),
        "skirmish.ron must author its 30x30x4 grid_size",
    );
    assert_eq!(
        situation.gangers.len(),
        3,
        "skirmish.ron must keep its three placed gangers (GTW-414 gang-name refs intact)",
    );

    // NO inline terrain remains (the GTW-433 migration removed all of it).
    assert!(
        situation.walls.is_empty(),
        "skirmish.ron must author no walls"
    );
    assert!(
        situation.scatter.is_empty(),
        "skirmish.ron must author no scatter",
    );
    assert!(
        situation.slabs.is_empty(),
        "skirmish.ron must author no slabs"
    );
    assert!(
        situation.floors.is_empty(),
        "skirmish.ron must author no floor overrides",
    );
    assert!(
        situation.vertical_links.is_empty(),
        "skirmish.ron must author no vertical_links (they are terrain)",
    );
    assert!(
        situation.default_floor.is_empty(),
        "skirmish.ron must author no default_floor (procgen supplies it)",
    );
}

/// C2/C3: a theme+size-only situation procgen-generates its terrain at Generation and the
/// walk REACHES `BattleScapeState::BattleRunning` with a POPULATED `TerrainIndex`.
///
/// Pin: the authored situation has ZERO terrain, so a non-empty `TerrainIndex` proves the
/// terrain came from the LIVE procgen path (`generate_level` → merge → `setup_battle`), not
/// from the authored situation. If the procgen trigger were dead (an uncalled function), the
/// battle would build empty terrain and the count assertion would fail; if Generation
/// regressed, the walk would never reach `BattleRunning`.
#[test]
fn procgen_battle_reaches_running_with_populated_terrain() {
    let (situation, gangs) = theme_size_only_situation();
    let mut app = procgen_walk_app(situation, gangs);

    assert!(
        drive_past_menu(&mut app),
        "the walk should reach RunningState::Menu within {BUDGET} updates; last observed \
         RunningState was {:?}",
        running_state(&app),
    );

    let reached_running = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    );
    assert!(
        reached_running,
        "a theme+size-only situation should procgen its terrain and reach BattleRunning within \
         {BUDGET} updates; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    // The battle terrain is POPULATED — the procgen path produced terrain entities (walls /
    // scatter / slabs indexed in the TerrainIndex). The authored situation had none, so any
    // entry can only be procgen's.
    let terrain = app.world().get_resource::<TerrainIndex>();
    assert!(
        terrain.is_some(),
        "a live battle must insert a TerrainIndex resource at setup",
    );
    let Some(terrain) = terrain else {
        return;
    };
    assert!(
        !terrain.is_empty(),
        "the procgen-generated battle must have a POPULATED TerrainIndex (the authored \
         situation had zero terrain, so a non-empty index proves the procgen path ran); the \
         index was empty",
    );
}
