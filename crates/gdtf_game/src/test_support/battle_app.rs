//! Drive a headless app into `BattleRunning` with a seeded situation.

use bevy::{
    app::App,
    state::state::{NextState, State},
    time::TimeUpdateStrategy,
};
use cobalt_test_utils::{MinimalTestAppBuilder, advance_until};
use gdtf_battle_sim::{
    effects::fields::FieldDefRegistry,
    rng::BattleSeed,
    situation::{PlacedGanger, Situation},
    test_support::{
        fixtures, test_armor_registry, test_gang_registry, test_melee_weapon_registry,
        test_weapon_registry,
    },
    tuning::{CombatTuning, GangerStatTuning},
};
use gdtf_content_families::situation::LoadedSituation;
use gdtf_ui::theme::default_theme;

use super::register::register_headless;
use crate::states::{
    AppState, BattleScapeState, RunningState,
    running::game::battlescape::generation::battle_sim::PreplacedGangers,
};

/// Fixed steps a frame runs, so the descent advances per frame and never off the real clock.
const ONE_STEP_A_FRAME: u32 = 1;

/// Builds a `MinimalPlugins` app already in [`BattleScapeState::BattleRunning`].
#[derive(Debug, Clone)]
pub struct BattleAppBuilder {
    situation:  Situation,
    placements: Vec<PlacedGanger>,
    seed:       Option<BattleSeed>,
}

impl BattleAppBuilder {
    /// Two-ganger fixture situation and its placements, no fixed seed.
    #[must_use]
    pub fn new() -> Self {
        let (situation, placements) = fixtures::two_ganger();
        Self {
            situation,
            placements,
            seed: None,
        }
    }

    /// Override the situation and the gangers placed on it.
    #[must_use]
    pub fn with_situation(mut self, built: (Situation, Vec<PlacedGanger>)) -> Self {
        let (situation, placements) = built;
        self.situation = situation;
        self.placements = placements;
        self
    }

    /// Fix the battle seed.
    #[must_use]
    pub const fn with_seed(mut self, seed: BattleSeed) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Drive into `BattleRunning`.
    pub fn build(self) -> App {
        let mut app = MinimalTestAppBuilder::new_with_scene_support(register_headless)
            .starting_in(AppState::Running)
            .build();

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
            .insert_resource(LoadedSituation::new(self.situation));
        app.world_mut()
            .insert_resource(PreplacedGangers::new(self.placements));
        if let Some(seed) = self.seed {
            app.world_mut().insert_resource(seed);
        }
        app.insert_resource(TimeUpdateStrategy::FixedTimesteps(ONE_STEP_A_FRAME));

        advance_until(&mut app, |app| {
            running_state(app) == Some(RunningState::Menu)
        });
        app.world_mut()
            .resource_mut::<NextState<RunningState>>()
            .set(RunningState::Options);

        advance_until(&mut app, |app| {
            running_state(app) == Some(RunningState::Options)
        });
        app.world_mut()
            .resource_mut::<NextState<RunningState>>()
            .set(RunningState::Game);

        advance_until(&mut app, |app| {
            battlescape_state(app) == Some(BattleScapeState::BattleRunning)
        });
        app
    }
}

impl Default for BattleAppBuilder {
    fn default() -> Self {
        Self::new()
    }
}

fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}
