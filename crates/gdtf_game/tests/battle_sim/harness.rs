use bevy::state::state::State;
use cobalt_test_utils::{MinimalTestAppBuilder, advance_until};
use gdtf_battle_sim::{
    injuries::InjuryRegistry,
    situation::Situation,
    test_support::{
        SituationBuilder, ganger_at, key, test_armor_registry, test_melee_weapon_registry,
        test_weapon_registry,
    },
    tuning::CombatTuning,
};
use gdtf_content_families::situation::LoadedSituation;
use gdtf_game::test_support::{BattleScapeState, RunningState};
use gdtf_ui::theme::default_theme;

/// Fixed steps a frame runs, so the descent advances per frame and never off the real clock.
const ONE_STEP_A_FRAME: u32 = 1;

pub(crate) fn two_ganger_situation() -> Situation {
    SituationBuilder::new()
        .with_gangers([ganger_at(key(5, 6, 0), 0), ganger_at(key(7, 8, 0), 1)])
        .build()
}

pub(crate) fn battlescape_state(app: &bevy::app::App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

pub(crate) fn running_state(app: &bevy::app::App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

pub(crate) fn drive_past_menu(app: &mut bevy::app::App) {
    advance_until(app, |app| running_state(app) == Some(RunningState::Menu));
    app.world_mut()
        .resource_mut::<bevy::state::state::NextState<RunningState>>()
        .set(RunningState::Options);
    advance_until(app, |app| running_state(app) == Some(RunningState::Options));
    app.world_mut()
        .resource_mut::<bevy::state::state::NextState<RunningState>>()
        .set(RunningState::Game);
}

pub(crate) fn walk_app(situation: Option<Situation>) -> bevy::app::App {
    let mut app =
        MinimalTestAppBuilder::new_with_scene_support(gdtf_game::test_support::register_headless)
            .default_start()
            .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(test_weapon_registry());
    app.world_mut()
        .insert_resource(test_melee_weapon_registry());
    app.world_mut().insert_resource(test_armor_registry());
    app.world_mut().insert_resource(InjuryRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::test_support::test_gang_registry());
    app.world_mut()
        .insert_resource(LoadedSituation::new(situation.unwrap_or_default()));
    app.insert_resource(bevy::time::TimeUpdateStrategy::FixedTimesteps(
        ONE_STEP_A_FRAME,
    ));
    app
}

pub(crate) fn drive_to_generation(app: &mut bevy::app::App) {
    drive_past_menu(app);
    advance_until(app, |app| {
        battlescape_state(app) == Some(BattleScapeState::Generation)
    });
}
