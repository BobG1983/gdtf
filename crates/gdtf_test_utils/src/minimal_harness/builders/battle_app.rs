//! The [`BattleAppBuilder`] — a one-call headless app already driven to a live
//! battle (`BattleScapeState::BattleRunning`), consolidating the per-app-test
//! setup-and-drive sequence the integration tests currently inline.

use bevy::{
    app::App,
    state::state::{NextState, State},
};
use gdtf_app::test_support::{AppState, BattleScapeState, LoadedSituation, RunningState};
use gdtf_battle_sim::{
    FieldDefRegistry,
    rng::BattleSeed,
    situation::Situation,
    test_support::{
        fixtures, test_armor_registry, test_gang_registry, test_melee_weapon_registry,
        test_weapon_registry,
    },
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
///
/// Use [`with_seed`](BattleAppBuilder::with_seed) to pin the [`BattleSeed`] for
/// deterministic cross-run replay; without it, `Generation` uses `resolve_root_seed()`
/// (env var or wall-clock), producing a different stream each run.
#[derive(Debug, Clone)]
pub struct BattleAppBuilder {
    /// The authored battlefield the Generation setup pours into the world.
    situation: Situation,
    /// Optional fixed seed injected as a `BattleSeed` resource before Generation runs;
    /// `None` means the normal `resolve_root_seed()` path (env var / wall-clock).
    seed:      Option<BattleSeed>,
}

impl BattleAppBuilder {
    /// A fresh builder over the default [`fixtures::two_ganger`] situation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            situation: fixtures::two_ganger(),
            seed:      None,
        }
    }

    /// Use a specific authored [`Situation`] instead of the default
    /// [`fixtures::two_ganger`].
    #[must_use]
    pub fn with_situation(mut self, situation: Situation) -> Self {
        self.situation = situation;
        self
    }

    /// Pin the [`BattleSeed`] for deterministic cross-run replay.
    ///
    /// When set, the seed is pre-inserted as a [`BattleSeed`] resource before the
    /// app enters `Generation`; `request_battle_setup` reads it and bypasses the
    /// normal `resolve_root_seed()` (env var / wall-clock) path. This guarantees
    /// that two builder runs with the same fixed seed produce the identical RNG
    /// stream and the identical combat outcomes — required for determinism tests.
    ///
    /// Without this, `Generation` calls `resolve_root_seed()`, which is different
    /// each run (unless `GDTF_BATTLE_SEED` is set in the environment).
    #[must_use]
    pub const fn with_seed(mut self, seed: BattleSeed) -> Self {
        self.seed = Some(seed);
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
        // GTW-505: the melee weapon registry (with the `fists` default) — without it
        // `setup_battle_on_request` fails closed (no MeleeWeaponRegistry) and spawns no
        // gangers; fixture gangers author no melee weapon, so each resolves to `fists`.
        app.world_mut()
            .insert_resource(test_melee_weapon_registry());
        app.world_mut().insert_resource(test_armor_registry());
        // GTW-545: the area-damage-field catalog. Empty by default — the standard fixtures
        // author no `fields:`, so an empty catalog seeds an empty FieldRegistry at setup; a
        // fixture that DID author a field would resolve its key against this. Present so
        // `setup_battle_on_request`'s `Option<Res<FieldDefRegistry>>` reads it.
        app.world_mut().insert_resource(FieldDefRegistry::default());
        // GTW-414/415: the canonical test gang registry every standard fixture's placed
        // gangers resolve their (gang, member) refs against. Built from the SAME
        // `build_with_gangs` split the fixtures use, so it can never drift from what
        // `ganger_at` / the default builder produce; without it the v2
        // `setup_battle_on_request` fails closed with GangNotFound and spawns no gangers.
        app.world_mut().insert_resource(test_gang_registry());
        // The authored battlefield the Generation setup pours into the world.
        app.world_mut()
            .insert_resource(LoadedSituation::new(self.situation));
        // If a fixed seed was requested, pre-inject it so `request_battle_setup`
        // bypasses `resolve_root_seed()` (wall-clock) and uses this value instead —
        // the only way to guarantee cross-run replay determinism in headless tests.
        if let Some(seed) = self.seed {
            app.world_mut().insert_resource(seed);
        }

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
