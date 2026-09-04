//! Loading screen: shows in `Generation`, then despawns when `AnimateIn` begins.
use bevy::{ecs::entity::Entity, prelude::*, state::state::State, ui::GlobalZIndex};
use gdtf_battle_sim::{injuries::InjuryRegistry, tuning::CombatTuning, weapon::WeaponRegistry};
use gdtf_game::test_support::{
    AppState, BattleScapeState, GameState, LoadingScreenRoot, RunningState,
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// Fixed steps a frame runs, so the descent advances per frame and never off the real clock.
const ONE_STEP_A_FRAME: u32 = 1;

const TOP_HUD_Z: i32 = 20;

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

fn all_with<M: Component>(app: &mut App) -> Vec<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    q.iter(app.world()).collect()
}

fn single_with<M: Component>(app: &mut App) -> Option<Entity> {
    match all_with::<M>(app).as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

fn app_driven_into_game() -> App {
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

    advance_until(&mut app, |app| {
        running_state(app) == Some(RunningState::Menu)
    });
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    app
}

#[test]
fn loading_screen_shows_in_generation_then_despawns_on_animate_in() {
    let mut app = app_driven_into_game();

    advance_until(&mut app, |app| {
        battlescape_state(app) == Some(BattleScapeState::Generation)
    });

    assert!(
        single_with::<LoadingScreenRoot>(&mut app).is_some(),
        "exactly one LoadingScreenRoot must exist while in Generation (the loading screen covers \
         the assembly phase)",
    );

    advance_until(&mut app, |app| {
        battlescape_state(app) == Some(BattleScapeState::AnimateIn)
    });

    assert!(
        single_with::<LoadingScreenRoot>(&mut app).is_none(),
        "the loading screen must be despawned after the transition to AnimateIn \
         (DespawnOnExit(Generation))",
    );
}

#[test]
fn loading_screen_z_is_above_the_hud_band() {
    let mut app = app_driven_into_game();

    advance_until(&mut app, |app| {
        battlescape_state(app) == Some(BattleScapeState::Generation)
    });

    let root = single_with::<LoadingScreenRoot>(&mut app);
    assert!(
        root.is_some(),
        "exactly one LoadingScreenRoot must exist while in Generation",
    );
    let Some(root) = root else {
        return;
    };
    let z = app.world().get::<GlobalZIndex>(root);
    assert!(
        z.is_some_and(|z| z.0 > TOP_HUD_Z),
        "the loading screen root must carry a GlobalZIndex strictly above the HUD band ({TOP_HUD_Z}) \
         so its opaque fill occludes any partial level beneath it (AC2); was {z:?}",
    );

    assert_eq!(
        app.world()
            .get_resource::<State<GameState>>()
            .map(|state| *state.get()),
        Some(GameState::BattleScape),
        "the loading screen is shown inside GameState::BattleScape",
    );
}
