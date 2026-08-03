use gdtf_app::test_support::{AppState, GameState, RunningState};
use gdtf_battle_presenter::TopDownRendererActive;
use gdtf_battle_sim::{injuries::InjuryRegistry, tuning::CombatTuning, weapon::WeaponRegistry};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

const BUDGET: u32 = 96;

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
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
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
    app
}

fn drive_past_menu(app: &mut bevy::app::App) -> bool {
    let reached = advance_until(
        app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    if !reached {
        return false;
    }
    app.world_mut()
        .resource_mut::<bevy::state::state::NextState<RunningState>>()
        .set(RunningState::Options);
    let reached_options = advance_until(
        app,
        |app| running_state(app) == Some(RunningState::Options),
        BUDGET,
    );
    if reached_options {
        app.world_mut()
            .resource_mut::<bevy::state::state::NextState<RunningState>>()
            .set(RunningState::Game);
    }
    reached_options
}

fn drive_to_battlescape(app: &mut bevy::app::App) -> bool {
    if !drive_past_menu(app) {
        return false;
    }
    advance_until(
        app,
        |app| game_state(app) == Some(GameState::BattleScape),
        BUDGET,
    )
}

#[test]
fn battlescape_scene_runs_the_presenter_plugin_build() {
    let mut app = presenter_app();
    assert!(
        drive_to_battlescape(&mut app),
        "the walk should descend to GameState::BattleScape within {BUDGET} updates; last observed \
         GameState was {:?}",
        game_state(&app),
    );
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
