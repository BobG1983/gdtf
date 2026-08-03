//! 1. [`skirmish_ron_authors_no_terrain`] — the SHIPPED `assets/content/situations/skirmish.ron`
use bevy::{app::App, prelude::NextState, state::state::State};
use gdtf_app::test_support::{AppState, BattleScapeState, RunningState};
use gdtf_battle_sim::{
    level::{PrefabRegistry, ThemeUuid},
    situation::Situation,
    terrain::entity::TerrainIndex,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until};

const BUDGET: u32 = 512;

const fn industrial_hive_theme() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a90_0001))
}

fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

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

    assert_eq!(
        situation.theme,
        industrial_hive_theme(),
        "skirmish.ron must author its theme (the IndustrialHive ThemeUuid)",
    );
    assert!(
        !situation.rosters.is_empty(),
        "skirmish.ron must author its roster members (GTW-744 gang-name refs, no cells)",
    );
    assert!(
        situation.gangers.is_empty(),
        "skirmish.ron must author NO placed gangers (GTW-744: zero authored cells; the deploy \
         step derives them)",
    );

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
        *situation.default_floor.is_nil(),
        "skirmish.ron must author no default_floor (procgen supplies it)",
    );
}

fn prefab_len(app: &App) -> Option<usize> {
    app.world()
        .get_resource::<PrefabRegistry>()
        .map(PrefabRegistry::len)
}

#[test]
fn procgen_battle_reaches_running_with_populated_terrain() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

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

    let before = prefab_len(&app);
    assert!(
        matches!(before, Some(n) if n > 0),
        "the REAL Load resolve must POPULATE a non-empty PrefabRegistry from shipped content \
         BEFORE the battle generates (it must NOT be a hand-seeded registry); registry len was \
         {before:?}",
    );

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
        "a theme+size-only situation should procgen its terrain and reach \
         BattleRunning within {BUDGET} updates; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );

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
         situation had zero terrain, so a non-empty index proves the procgen path ran \
         against the real-resolved PrefabRegistry); the index was empty",
    );
}
