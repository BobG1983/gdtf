//! The menu-resting headless app the deep-phase case drives into a battle (GTW-943).
//!
//! It used to live in the T9 `start_battle` suite, which went with the `StartBattle`
//! request it drove. The fixture itself did not: descending the real state machine from the
//! menu into `BattleRunning` is what makes `app.phase`'s nested levels worth reading, and
//! that descent is the menu's own `StartBattleRequested` message — the SAME one the
//! Battlescape button writes, one step earlier on the path the deleted request used to join.

use std::sync::mpsc;

use bevy::{app::App, state::state::State};
use gdtf_app::test_support::{
    AppState, LoadedSituation, NetQaPlugin, RunningState, StartBattleRequested,
};
use gdtf_battle_sim::{
    effects::fields::FieldDefRegistry,
    test_support::{
        fixtures, test_armor_registry, test_gang_registry, test_melee_weapon_registry,
        test_weapon_registry,
    },
    tuning::{CombatTuning, GangerStatTuning},
};
use gdtf_net_qa_transport::IncomingRequest;
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// The per-milestone frame budget for the deep menu → battle descent.
pub(crate) const DRIVE_BUDGET: u32 = 128;

/// Reads the current [`RunningState`] if it is active.
fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Build a headless app resting at [`RunningState::Menu`] with the REAL `net_qa` router
/// wired to an injected inbox, and every persistent `Load` resource the deep descent needs
/// pre-seeded (a `MinimalPlugins` app has no `AssetServer` to resolve them).
///
/// Returns the app and the sender a test pushes requests on, exactly as the listener thread
/// would.
pub(crate) fn menu_app_with_net_qa() -> (App, mpsc::Sender<IncomingRequest>) {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    // The persistent `Load` resources the machine traverses `Load` with, mirroring
    // `BattleAppBuilder` (the menu → battle harness): theme, tunings, and the weapon /
    // armor / gang / field registries the Generation setup pours the situation into, plus
    // the authored `LoadedSituation` itself.
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(GangerStatTuning::default());
    app.world_mut().insert_resource(test_weapon_registry());
    app.world_mut()
        .insert_resource(test_melee_weapon_registry());
    app.world_mut().insert_resource(test_armor_registry());
    app.world_mut().insert_resource(FieldDefRegistry::default());
    app.world_mut().insert_resource(test_gang_registry());
    app.world_mut()
        .insert_resource(LoadedSituation::new(fixtures::two_ganger()));

    let (tx, rx) = mpsc::channel();
    app.add_plugins(NetQaPlugin::with_channels(rx));

    // Settle into the menu (and run one empty router pass).
    let rested = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Menu),
        DRIVE_BUDGET,
    );
    assert!(rested, "the harness must rest at RunningState::Menu");
    (app, tx)
}

/// Write the menu's own start-battle request — the message the Battlescape button writes.
pub(crate) fn request_battle(app: &mut App) {
    app.world_mut()
        .write_message(StartBattleRequested::new(None));
}
