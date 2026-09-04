use bevy::{
    app::App,
    state::state::{NextState, State},
};
use gdtf_battle_sim::{
    injuries::InjuryRegistry,
    situation::Situation,
    tuning::{CombatTuning, GangerStatTuning},
    weapon::WeaponRegistry,
};
use gdtf_content_families::situation::LoadedSituation;
use gdtf_game::test_support::{BattleScapeState, RunningState};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// Fixed steps a frame runs, so the descent advances per frame and never off the real clock.
const ONE_STEP_A_FRAME: u32 = 1;

pub(crate) fn walk_app_with_theme() -> App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .default_start()
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(GangerStatTuning::default());
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
    app.world_mut()
        .insert_resource(LoadedSituation::new(Situation::default()));
    app.insert_resource(bevy::time::TimeUpdateStrategy::FixedTimesteps(
        ONE_STEP_A_FRAME,
    ));
    app
}

pub(crate) fn running_state(app: &bevy::app::App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

pub(crate) fn battlescape_state(app: &bevy::app::App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

pub(crate) fn drive_past_menu(app: &mut App) {
    advance_until(app, |app| running_state(app) == Some(RunningState::Menu));
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Options);
    advance_until(app, |app| running_state(app) == Some(RunningState::Options));
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
}
