//! The [`BattleAppBuilder`] — a one-call headless app already driven to a live
//! battle (`BattleScapeState::BattleRunning`), consolidating the per-app-test
//! setup-and-drive sequence the integration tests currently inline.

use bevy::{
    app::App,
    state::state::{NextState, State},
};
use gdtf_app::test_support::{AppState, BattleScapeState, LoadedSituation, RunningState};
use gdtf_battle_sim::{
    situation::Situation,
    test_support::{fixtures, test_armor_registry, test_weapon_registry},
    tuning::{CombatTuning, GangerStatTuning},
};
use gdtf_ui::theme::default_theme;

use crate::{GdtfTestAppBuilder, advance_until};

/// The per-milestone frame budget the drive uses — generous enough for the deep
/// walk down into the battlescape (each leaf scene spends a couple of `FixedUpdate`
/// ticks plus its transition propagation), but bounded so a machine that never
/// reaches the predicate fails instead of hanging. The same `96` the per-app battle
/// integration tests inline.
const DRIVE_BUDGET: u32 = 96;

/// Builds a headless GDTF [`App`] already rested in
/// [`BattleScapeState::BattleRunning`] with the sim constructed — the
/// consolidation of the "seed the persistent `Load` resources, insert a
/// [`LoadedSituation`], drive past the menu, descend to `BattleRunning`" sequence
/// every per-app battle integration test currently inlines.
///
/// It wraps [`GdtfTestAppBuilder`] started at [`AppState::Running`] and inserts the
/// persistent `Load` resources a `MinimalPlugins` app has no `AssetServer` to load
/// (the UI [`default_theme`], [`CombatTuning::default`], and the canonical
/// [`test_weapon_registry`] / [`test_armor_registry`] from
/// [`gdtf_battle_sim::test_support`]) plus the chosen [`Situation`] (default
/// [`fixtures::two_ganger`]) as a [`LoadedSituation`]. [`build`](BattleAppBuilder::build)
/// then drives the real state machine to a live battle and returns the [`App`].
#[derive(Debug, Clone)]
pub struct BattleAppBuilder {
    /// The authored battlefield the Generation setup pours into the world.
    situation: Situation,
}

impl BattleAppBuilder {
    /// A fresh builder over the default [`fixtures::two_ganger`] situation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            situation: fixtures::two_ganger(),
        }
    }

    /// Use a specific authored [`Situation`] instead of the default
    /// [`fixtures::two_ganger`].
    #[must_use]
    pub fn with_situation(mut self, situation: Situation) -> Self {
        self.situation = situation;
        self
    }

    /// Build the headless app and drive it to a live battle, returning the [`App`]
    /// rested in [`BattleScapeState::BattleRunning`].
    ///
    /// Returns `None` if the state machine does not reach the menu or
    /// `BattleRunning` within the drive budget (a genuinely broken drive), keeping
    /// the harness free of `unwrap`/`expect`/`panic` so a caller asserts the `Some`
    /// itself.
    #[must_use]
    pub fn build(self) -> Option<App> {
        let mut app = GdtfTestAppBuilder::new_with_scene_support()
            .starting_in(AppState::Running)
            .build();

        // Seed the persistent `Load` resources the machine traverses `Load` with —
        // a `MinimalPlugins` app has no `AssetServer` to resolve them from disk.
        app.world_mut().insert_resource(default_theme());
        app.world_mut().insert_resource(CombatTuning::default());
        // GTW-384: the persistent stat-derivation tuning the sim derives ganger stats from.
        app.world_mut().insert_resource(GangerStatTuning::default());
        app.world_mut().insert_resource(test_weapon_registry());
        app.world_mut().insert_resource(test_armor_registry());
        // The authored battlefield the Generation setup pours into the world.
        app.world_mut()
            .insert_resource(LoadedSituation::new(self.situation));

        // Stand in for the player at the menu (it no longer auto-advances): advance
        // until the menu rests, then queue Menu -> Options to descend toward the game.
        if !advance_until(
            &mut app,
            |app| running_state(app) == Some(RunningState::Menu),
            DRIVE_BUDGET,
        ) {
            return None;
        }
        app.world_mut()
            .resource_mut::<NextState<RunningState>>()
            .set(RunningState::Options);

        // Descend to a live battle: the app's OnEnter(Generation) sends
        // SetupBattleRequested and the sim's setup_battle spawns the gangers via
        // Commands, so resting at BattleRunning means the battle is constructed.
        if !advance_until(
            &mut app,
            |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
            DRIVE_BUDGET,
        ) {
            return None;
        }
        Some(app)
    }
}

impl Default for BattleAppBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Reads the current [`RunningState`] if it is active.
fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Reads the current [`BattleScapeState`] if it is active.
fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}
