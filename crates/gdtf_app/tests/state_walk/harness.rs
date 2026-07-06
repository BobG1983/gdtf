//! The themed walk app, the menu driver, and shared state readers.

use bevy::{
    app::App,
    state::state::{NextState, State},
};
use gdtf_app::test_support::{BattleScapeState, LoadedSituation, RunningState};
use gdtf_battle_sim::{
    injuries::InjuryRegistry,
    situation::Situation,
    tuning::{CombatTuning, GangerStatTuning},
    weapon::WeaponRegistry,
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// A budget large enough for the whole deep walk (each leaf scene spends a
/// couple of `FixedUpdate` ticks plus its state-transition propagation), but
/// still bounded so a machine that never terminates fails instead of hanging.
pub(crate) const WALK_BUDGET: u32 = 64;

/// Builds the default-start headless walk app and seeds the four resources the
/// `Load` scene now requires (GTW-143 / GTW-206 / GTW-257 / GTW-261).
///
/// `Load` no longer advances on a frame-1 shortcut: it leaves only once a
/// [`GdtfTheme`](gdtf_ui::theme::GdtfTheme), a [`CombatTuning`], a
/// [`WeaponRegistry`], AND a [`LoadedSituation`] are all present, which the running
/// app resolves from the loose theme/tuning/weapons/situation RON via the
/// `AssetServer` (GTW-206 added the tuning, GTW-257 the registry, GTW-261 the
/// situation as required gate resources). The `MinimalPlugins` walk app has **no**
/// `AssetServer`, so this pre-inserts all four (standing in for the resolved loads)
/// so the walk can traverse `Load` and exercise the deep transition graph this file
/// is about. All four are the deliberate state-scoped-resource exceptions that
/// persist, so seeding them before the walk is faithful to how the real app carries
/// them forward.
pub(crate) fn walk_app_with_theme() -> App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .default_start()
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    // GTW-384: the Load→Intro gate also requires a GangerStatTuning (the sim derives
    // ganger stats from it); the default clears the gate.
    app.world_mut().insert_resource(GangerStatTuning::default());
    // GTW-257: the Load→Intro gate also requires a WeaponRegistry (the deep walk uses
    // the empty-default situation, so an empty registry clears the gate).
    app.world_mut().insert_resource(WeaponRegistry::default());
    // GTW-505: the Load->Intro gate also requires a MeleeWeaponRegistry (empty-default
    // seed stands in for the asset-less resolve, mirroring the WeaponRegistry seed above).
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::MeleeWeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::equipment::attachments::AttachmentRegistry::default());
    // GTW-269: the Load→Intro gate also requires an ArmorRegistry; empty clears it (the
    // registry is dormant this slice — the setup does not read it yet).
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    // GTW-415: the Load→Intro gate also requires a GangRegistry; empty clears it (a
    // real battle would resolve placed gangers against the loaded gangs folder).
    app.world_mut()
        .insert_resource(gdtf_battle_sim::ganger::GangRegistry::default());
    // GTW-489: the NEW gate-blocking PrefabRegistry; empty clears it.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::PrefabRegistry::default());
    // GTW-487: the NEW gate-blocking TerrainDefRegistry + UuidThemeRegistry.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::terrain::def::TerrainDefRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::UuidThemeRegistry::default());
    // GTW-261: the Load→Intro gate now also requires a LoadedSituation (the
    // empty-battle-race fix). The headless walk has no AssetServer to resolve one, so
    // seed the empty default beside the other three — symmetric with theme/tuning/weapons.
    app.world_mut()
        .insert_resource(LoadedSituation::new(Situation::default()));
    app
}

/// Reads the current [`RunningState`] if [`AppState::Running`] is active.
pub(crate) fn running_state(app: &bevy::app::App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Reads the current [`BattleScapeState`] if [`GameState::BattleScape`] is active.
pub(crate) fn battlescape_state(app: &bevy::app::App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// Stands in for the player at the menu: advances the walk until
/// [`RunningState::Menu`] is reached, then queues the `Menu → Options`
/// transition (the menu no longer auto-advances since GTW-121).
///
/// Returns whether `Menu` was reached and the transition queued within budget.
/// After this returns `true`, one more `update()` (driven by the caller's
/// `advance_until`) applies the queued `NextState` and the rest of the chain
/// continues through its scaffolds.
pub(crate) fn drive_past_menu(app: &mut App) -> bool {
    let reached_menu = advance_until(
        app,
        |app| running_state(app) == Some(RunningState::Menu),
        WALK_BUDGET,
    );
    if reached_menu {
        app.world_mut()
            .resource_mut::<NextState<RunningState>>()
            .set(RunningState::Options);
    }
    reached_menu
}
