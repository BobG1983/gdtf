use bevy::state::state::State;
use gdtf_app::test_support::{BattleScapeState, LoadedSituation, RunningState};
use gdtf_battle_sim::{
    injuries::InjuryRegistry,
    situation::Situation,
    test_support::{
        SituationBuilder, ganger_at, key, test_armor_registry, test_melee_weapon_registry,
        test_weapon_registry,
    },
    tuning::CombatTuning,
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

pub(crate) const BUDGET: u32 = 96;

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

pub(crate) fn drive_past_menu(app: &mut bevy::app::App) -> bool {
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

pub(crate) fn walk_app(situation: Option<Situation>) -> bevy::app::App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
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
    app
}

pub(crate) fn drive_to_generation(app: &mut bevy::app::App) -> bool {
    if !drive_past_menu(app) {
        return false;
    }
    advance_until(
        app,
        |app| battlescape_state(app) == Some(BattleScapeState::Generation),
        BUDGET,
    )
}
