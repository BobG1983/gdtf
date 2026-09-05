//! 1. [`skirmish_ron_authors_no_terrain`] — the SHIPPED `assets/content/situations/skirmish.ron`
use bevy::{app::App, prelude::NextState, state::state::State};
use cobalt_test_utils::{LoadTestAppBuilder, advance_until};
use gdtf_battle_sim::{
    level::{PrefabRegistry, ThemeUuid},
    situation::Situation,
    terrain::entity::TerrainIndex,
};
use gdtf_game::test_support::{AppState, BattleScapeState, RunningState};

/// Fixed steps a frame runs, so the descent advances per frame and never off the real clock.
const ONE_STEP_A_FRAME: u32 = 1;

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
    let ron = include_str!("../../../../assets/content/situations/skirmish.ron");
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
        "skirmish.ron must author its roster members (gang-name refs, no cells)",
    );
    assert!(
        situation.gangers.is_empty(),
        "skirmish.ron must author NO placed gangers (zero authored cells; the deploy step derives them)",
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
    let mut app =
        LoadTestAppBuilder::new(gdtf_game::test_support::register_scenes_with_default_plugins)
            .starting_in(AppState::Load)
            .build();

    app.insert_resource(bevy::time::TimeUpdateStrategy::FixedTimesteps(
        ONE_STEP_A_FRAME,
    ));
    advance_until(&mut app, |app| {
        running_state(app) == Some(RunningState::Menu)
    });

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
    advance_until(&mut app, |app| {
        battlescape_state(app) == Some(BattleScapeState::BattleRunning)
    });

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
