//! Presenter foundation: battlescape scene runs the top-down presenter plugin.
use cobalt_test_utils::{MinimalTestAppBuilder, advance_until};
use gdtf_battle_presenter::TopDownRendererActive;
use gdtf_battle_sim::{injuries::InjuryRegistry, tuning::CombatTuning, weapon::WeaponRegistry};
use gdtf_game::test_support::{AppState, GameState, RunningState};
use gdtf_ui::theme::default_theme;

/// Fixed steps a frame runs, so the descent advances per frame and never off the real clock.
const ONE_STEP_A_FRAME: u32 = 1;

fn game_state(app: &bevy::app::App) -> Option<GameState> {
    app.world()
        .get_resource::<bevy::state::state::State<GameState>>()
        .map(|state| *state.get())
}

fn running_state(app: &bevy::app::App) -> Option<RunningState> {
    app.world()
        .get_resource::<bevy::state::state::State<RunningState>>()
        .map(|state| *state.get())
}

fn presenter_app() -> bevy::app::App {
    let mut app =
        MinimalTestAppBuilder::new_with_scene_support(gdtf_game::test_support::register_headless)
            .starting_in(AppState::Running)
            .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(WeaponRegistry::default());
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
    app
}

fn drive_past_menu(app: &mut bevy::app::App) {
    advance_until(app, |app| running_state(app) == Some(RunningState::Menu));
    app.world_mut()
        .resource_mut::<bevy::state::state::NextState<RunningState>>()
        .set(RunningState::Options);
    advance_until(app, |app| running_state(app) == Some(RunningState::Options));
    app.world_mut()
        .resource_mut::<bevy::state::state::NextState<RunningState>>()
        .set(RunningState::Game);
}

fn drive_to_battlescape(app: &mut bevy::app::App) {
    drive_past_menu(app);
    advance_until(app, |app| game_state(app) == Some(GameState::BattleScape));
}

#[test]
fn battlescape_scene_runs_the_presenter_plugin_build() {
    let mut app = presenter_app();
    drive_to_battlescape(&mut app);
    assert_eq!(
        game_state(&app),
        Some(GameState::BattleScape),
        "the walk must rest inside GameState::BattleScape",
    );
    assert!(
        app.world()
            .get_resource::<TopDownRendererActive>()
            .is_some(),
        "BattlePresenterPlugin (default TopDown) build must have run inside the real scene stack — \
         the TopDownRendererActive marker is present once BattleScape is active",
    );
}
