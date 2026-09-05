use bevy::{ecs::entity::Entity, platform::collections::HashSet, prelude::*, state::state::State};
use cobalt_test_utils::{MinimalTestAppBuilder, advance_until};
use gdtf_battle_presenter::{DrawnPosition, Played, ShownSquadVisibility};
use gdtf_battle_sim::{
    ganger::{GangerName, Position},
    injuries::InjuryRegistry,
    prelude::{Cell, CellLevel, Level},
    tuning::CombatTuning,
    visibility::SquadVisibility,
};
use gdtf_game::test_support::{AppState, BattleScapeState, RunningState};
use gdtf_ui::theme::default_theme;

/// Fixed steps a frame runs, so the descent advances per frame and never off the real clock.
const ONE_STEP_A_FRAME: u32 = 1;

pub(crate) fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

pub(crate) fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

pub(crate) fn battle_running_app() -> App {
    let mut app =
        MinimalTestAppBuilder::new_with_scene_support(gdtf_game::test_support::register_headless)
            .starting_in(AppState::Running)
            .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::WeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::MeleeWeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::equipment::attachments::AttachmentRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::ganger::GangRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::PrefabRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::terrain::def::TerrainDefRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::UuidThemeRegistry::default());
    app.insert_resource(bevy::time::TimeUpdateStrategy::FixedTimesteps(
        ONE_STEP_A_FRAME,
    ));

    advance_until(&mut app, |app| {
        running_state(app) == Some(RunningState::Menu)
    });
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    advance_until(&mut app, |app| {
        battlescape_state(app) == Some(BattleScapeState::BattleRunning)
    });
    light_the_fixture_cells(&mut app);
    app
}

/// A cell the screen is lighting, which the panel therefore reports acts on.
pub(crate) fn a_lit_cell(_app: &App) -> CellLevel {
    CellLevel::new(Cell::new(3, 3), Level::new(0))
}

/// Two different cells the screen is lighting, for a case that needs a move between them.
pub(crate) fn two_lit_cells(app: &App) -> (CellLevel, CellLevel) {
    (
        a_lit_cell(app),
        CellLevel::new(Cell::new(3, 5), Level::new(0)),
    )
}

/// A cell the screen is not lighting, which the panel therefore withholds acts on.
pub(crate) fn a_dark_cell(_app: &App) -> CellLevel {
    CellLevel::new(Cell::new(40, 40), Level::new(0))
}

// Give the squad a fog covering the fixture's lit cells and let the screen promote it.
fn light_the_fixture_cells(app: &mut App) {
    let (first, second) = two_lit_cells(app);
    let visible: HashSet<CellLevel> = [first, second].into_iter().collect();
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible.clone(), visible));
    for _ in 0..4 {
        app.update();
    }

    let dark = a_dark_cell(app);
    let Some(shown) = app.world().get_resource::<ShownSquadVisibility>() else {
        unreachable!("the presenter registers the fog shadow the panel reads");
    };
    assert!(
        *shown.visibility().is_cell_visible(&first)
            && *shown.visibility().is_cell_visible(&second)
            && !*shown.visibility().is_cell_visible(&dark),
        "the fixture's fog must reach the screen before a case plays anything: {first:?} and \
         {second:?} lit, {dark:?} dark",
    );
}

pub(crate) fn all_with<M: Component>(app: &mut App) -> Vec<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    q.iter(app.world()).collect()
}

pub(crate) fn line_texts<M: Component>(app: &mut App) -> Vec<String> {
    let entities = all_with::<M>(app);
    entities
        .into_iter()
        .filter_map(|e| app.world().get::<Text>(e).map(|t| t.as_str().to_owned()))
        .collect()
}

pub(crate) fn play<M: Message + Clone>(app: &mut App, fact: M) {
    let written = app.world_mut().write_message(Played::new(fact)).is_some();
    assert!(
        written,
        "the Played<{}> buffer must be registered by the presenter's playback registration",
        core::any::type_name::<M>(),
    );
}

/// A named ganger standing where the screen can see it, so the panel names its acts.
pub(crate) fn spawn_named(app: &mut App, name: &str) -> Entity {
    let at = a_lit_cell(app);
    spawn_named_at(app, name, at)
}

/// A named ganger standing where the screen cannot see it.
pub(crate) fn spawn_hidden(app: &mut App, name: &str) -> Entity {
    let at = a_dark_cell(app);
    spawn_named_at(app, name, at)
}

fn spawn_named_at(app: &mut App, name: &str, at: CellLevel) -> Entity {
    app.world_mut()
        .spawn((
            GangerName::new(name.to_owned()),
            DrawnPosition::seeded(Position::new(at)),
        ))
        .id()
}
