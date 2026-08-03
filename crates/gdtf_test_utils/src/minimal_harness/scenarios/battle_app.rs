use bevy::{
    app::App,
    state::state::{NextState, State},
};
use gdtf_app::test_support::{AppState, BattleScapeState, LoadedSituation, RunningState};
use gdtf_battle_sim::{
    effects::fields::FieldDefRegistry,
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

const DRIVE_BUDGET: u32 = 96;

#[derive(Debug, Clone)]
pub struct BattleAppBuilder {
        situation: Situation,
            seed:      Option<BattleSeed>,
}

impl BattleAppBuilder {
        #[must_use]
    pub fn new() -> Self {
        Self {
            situation: fixtures::two_ganger(),
            seed:      None,
        }
    }

            #[must_use]
    pub fn with_situation(mut self, situation: Situation) -> Self {
        self.situation = situation;
        self
    }

                                            #[must_use]
    pub const fn with_seed(mut self, seed: BattleSeed) -> Self {
        self.seed = Some(seed);
        self
    }

                                #[must_use]
    pub fn build(self) -> Option<App> {
        let mut app = GdtfTestAppBuilder::new_with_scene_support()
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
        if let Some(seed) = self.seed {
            app.world_mut().insert_resource(seed);
        }

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

        if !advance_until(
            &mut app,
            |app| running_state(app) == Some(RunningState::Options),
            DRIVE_BUDGET,
        ) {
            return None;
        }
        app.world_mut()
            .resource_mut::<NextState<RunningState>>()
            .set(RunningState::Game);

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
