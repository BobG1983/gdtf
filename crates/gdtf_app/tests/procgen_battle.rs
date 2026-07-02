//! GTW-433 / GTW-492 (situation procgen on the UUID v2 model): a theme+size-only situation
//! procgen-generates its terrain at `BattleScapeState::Generation` and reaches
//! `BattleScapeState::BattleRunning` with a PLAYABLE (terrain-populated) level — driven by the
//! REAL Load flow so the v2 prefab + theme + terrain registries are POPULATED from shipped
//! content (the GTW-489 resolve over the GTW-490 migrated `content/maps/` + `terrain/`), NOT
//! hand-seeded.
//!
//! Two tests:
//!
//! 1. [`skirmish_ron_authors_no_terrain`] — the SHIPPED `assets/content/situations/skirmish.ron`
//!    authors theme + `grid_size` + gangers and NO inline terrain (GTW-433 C1: the migration
//!    removed walls / scatter / slabs / `default_floor` / floors / `vertical_links`). Parses the
//!    real asset file directly (no app).
//! 2. [`procgen_battle_reaches_running_with_populated_terrain`] — the full REAL app walk
//!    (`GdtfLoadTestAppBuilder` → live `AssetServer` rooted at the workspace `assets/`) drives
//!    `Load` to completion, asserts the v2 [`PrefabRegistry2`] was POPULATED via the resolve
//!    branch (not-empty BEFORE the battle generates), then drives into the battle, REACHES
//!    `BattleRunning`, and asserts the battle's `TerrainIndex` is NON-EMPTY — terrain the
//!    authored situation did NOT contain, so it can only have come from the live procgen path
//!    on the v2 model (GTW-492 C4). Explicitly NOT a hand-seeded `PrefabRegistry2`: the registry
//!    is what the real Load resolve produced from shipped content.

use bevy::{app::App, prelude::NextState, state::state::State};
use gdtf_app::test_support::{AppState, BattleScapeState, RunningState};
use gdtf_battle_sim::{
    level::{PrefabRegistry2, ThemeUuid},
    situation::Situation,
    terrain::entity::TerrainIndex,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until};

/// A generous budget for the real `DefaultPlugins` async asset loads + the full state descent
/// under contention (the `battle_end_at_impact.rs` / `real_battle_panel.rs` precedent).
const BUDGET: u32 = 512;

/// The migrated `IndustrialHive` [`ThemeUuid`] the shipped `skirmish.ron` authors in its
/// `theme` field (GTW-490 migrated key, reconciled into the canonical `theme` by GTW-491).
const fn industrial_hive_theme() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a90_0001))
}

/// Reads the current [`RunningState`] if it is active.
fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Reads the current [`BattleScapeState`] if it is active.
fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// C1 (GTW-433): the SHIPPED `skirmish.ron` authors theme + `grid_size` + gangers and NO
/// inline terrain.
///
/// Deserialises the real asset file (the same RON the loader reads) and asserts every terrain
/// list is empty and `default_floor` is unset, while theme / `grid_size` / gangers are present.
/// Pin: re-authoring any terrain entry into `skirmish.ron` (un-migrating it) turns this red.
#[test]
fn skirmish_ron_authors_no_terrain() {
    let ron = include_str!("../../../assets/content/situations/skirmish.ron");
    let parsed = ron::from_str::<Situation>(ron);
    assert!(
        parsed.is_ok(),
        "the shipped skirmish.ron must deserialize as a Situation: {:?}",
        parsed.as_ref().err(),
    );
    let Ok(situation) = parsed else {
        return;
    };

    // Theme + size + gangers are authored (the kept fields). GTW-491: `theme` is now the
    // UUID-keyed IndustrialHive ThemeUuid (reconciled from the GTW-490 `theme_uuid`).
    assert_eq!(
        situation.theme,
        industrial_hive_theme(),
        "skirmish.ron must author its theme (the IndustrialHive ThemeUuid)",
    );
    assert!(
        !situation.gangers.is_empty(),
        "skirmish.ron must keep its placed gangers (GTW-414 gang-name refs intact)",
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
        situation.default_floor.is_nil(),
        "skirmish.ron must author no default_floor (procgen supplies it)",
    );
}

/// The number of prefabs the loaded v2 registry holds — `None` if the registry is absent.
fn prefab_v2_len(app: &App) -> Option<usize> {
    app.world()
        .get_resource::<PrefabRegistry2>()
        .map(PrefabRegistry2::len)
}

/// C4 (GTW-492): the REAL Load flow POPULATES the v2 prefab registry from shipped content, and
/// a theme+size-only situation then procgen-generates its terrain at Generation and REACHES
/// `BattleScapeState::BattleRunning` with a POPULATED `TerrainIndex`.
///
/// Drives `GdtfLoadTestAppBuilder` (a live `AssetServer` rooted at the workspace `assets/`, the
/// REAL Load scene) — so the [`PrefabRegistry2`] is built by the GTW-489 resolve from the
/// GTW-490 migrated `maps/industrial_hive/12x12/*.prefab_v2.ron` content, the
/// `UuidThemeRegistry` from `terrain/industrial_hive/*.terrain_theme.ron`, and the
/// `TerrainDefRegistry` from `terrain/industrial_hive/*.terrain_def.ron`. This is EXPLICITLY
/// NOT a hand-seeded registry: the assertion below proves the registry was populated via the
/// resolve branch (NON-EMPTY before the battle generates), so the procgen path reaches the real
/// populated registry.
///
/// Pin: the shipped `skirmish.ron` has ZERO inline terrain, so a non-empty `TerrainIndex` proves
/// the terrain came from the LIVE procgen path on the v2 model (`generate_level` → merge →
/// `setup_battle`), not from the authored situation. If procgen still read the old per-file
/// prefab / theme model, or the v2 registry resolved empty, the battle would build
/// empty terrain and the count assertion would fail.
#[test]
fn procgen_battle_reaches_running_with_populated_terrain() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Drive the REAL Load flow to completion (the menu rests once Load has resolved every
    // gate-blocking resource, including the v2 prefab + theme + terrain registries).
    let reached_menu = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    assert!(
        reached_menu,
        "the REAL Load flow must reach RunningState::Menu within {BUDGET} updates; last \
         observed RunningState was {:?}",
        running_state(&app),
    );

    // C4: the v2 prefab registry was POPULATED by the Load RESOLVE branch — NON-EMPTY BEFORE the
    // battle generates (the migrated player + enemy IndustrialHive deployment fragments + any
    // fill). This is the not-hand-seeded proof: the registry is purely what the real resolve
    // produced from shipped content. (`generate_level` has not run yet — we are still at Menu.)
    let before = prefab_v2_len(&app);
    assert!(
        matches!(before, Some(n) if n > 0),
        "the REAL Load resolve must POPULATE a non-empty PrefabRegistry2 from shipped content \
         BEFORE the battle generates (it must NOT be a hand-seeded registry); registry len was \
         {before:?}",
    );

    // Drive into the battle: the Generation system runs the v2 procgen against the populated
    // registries and the merged situation reaches BattleRunning.
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    let reached_running = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    );
    assert!(
        reached_running,
        "a theme+size-only situation should procgen its terrain on the v2 model and reach \
         BattleRunning within {BUDGET} updates; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    // The battle terrain is POPULATED — the procgen path produced terrain entities (walls /
    // scatter / slabs indexed in the TerrainIndex). The authored situation had none, so any
    // entry can only be procgen's, generated against the registry the real resolve populated.
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
         situation had zero terrain, so a non-empty index proves the v2 procgen path ran \
         against the real-resolved PrefabRegistry2); the index was empty",
    );
}
