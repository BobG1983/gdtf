//! Aim toggle: button active state tracks the selected shooter's aiming mode.
use bevy::{ecs::entity::Entity, prelude::*, state::state::State};
use gdtf_app::test_support::{AimToggleButton, AppState, BattleScapeState, RunningState};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    ganger::Aiming,
    injuries::InjuryRegistry,
    prelude::{Cell, CellLevel, Faction, Level, Position},
    tuning::CombatTuning,
    weapon::WeaponRegistry,
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::{SwitchState, theme::default_theme};

/// Fixed steps a frame runs, so the descent advances per frame and never off the real clock.
const ONE_STEP_A_FRAME: u32 = 1;

fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

fn walk_app() -> App {
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
    app.insert_resource(bevy::time::TimeUpdateStrategy::FixedTimesteps(
        ONE_STEP_A_FRAME,
    ));
    app
}

fn drive_to_battle_running(app: &mut App) {
    advance_until(app, |app| running_state(app) == Some(RunningState::Menu));
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    advance_until(app, |app| {
        battlescape_state(app) == Some(BattleScapeState::BattleRunning)
    });
}

fn battle_running_app() -> App {
    let mut app = walk_app();
    drive_to_battle_running(&mut app);
    app
}

fn single_with<M: Component>(app: &mut App) -> Option<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    let found: Vec<Entity> = q.iter(app.world()).collect();
    match found.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

fn aim_button_is_active(app: &mut App) -> bool {
    let Some(switch) = single_with::<AimToggleButton>(app) else {
        return false;
    };
    app.world()
        .get::<SwitchState>(switch)
        .is_some_and(|s| s.is_on())
}

fn spawn_and_select(app: &mut App, faction: Faction, aiming: bool) -> Entity {
    let ganger = app
        .world_mut()
        .spawn((
            Position::new(CellLevel::new(Cell::new(3, 3), Level::new(0))),
            faction,
            Aiming::new(aiming),
        ))
        .id();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    ganger
}

#[test]
fn aim_button_active_follows_selected_ganger_aiming() {
    let mut app = battle_running_app();

    let ganger = spawn_and_select(&mut app, Faction::new(0), true);
    app.update();
    assert!(
        aim_button_is_active(&mut app),
        "with the selected ganger aiming, the Aim switch must be SwitchState::On",
    );

    if let Some(mut aiming) = app.world_mut().get_mut::<Aiming>(ganger) {
        *aiming = Aiming::new(false);
    }
    app.update();
    assert!(
        !aim_button_is_active(&mut app),
        "with the selected ganger no longer aiming, the Aim switch must be SwitchState::Off",
    );
}

#[test]
fn no_selection_clears_active_marker() {
    let mut app = battle_running_app();

    spawn_and_select(&mut app, Faction::new(1), true);
    app.update();
    assert!(
        aim_button_is_active(&mut app),
        "the force-selected aiming ganger shows the Aim switch On before clearing",
    );

    app.world_mut().insert_resource(SelectedShooter::cleared());
    app.update();
    assert!(
        !aim_button_is_active(&mut app),
        "with no selection, the Aim switch must be SwitchState::Off (no stale ON state)",
    );
}
