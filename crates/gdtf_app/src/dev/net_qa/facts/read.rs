use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_sim::prelude::BattleInProgress;

use super::{BattleModel, GameFacts, StepperActivity};
use crate::{
    dev::net_qa::wire::{
        AfterMathPhaseNet, AppPhaseNet, BattleScapePhaseNet, GamePhaseNet, LifecyclePhaseNet,
        RunningPhaseNet,
    },
    states::{AfterMathState, AppState, BattleScapeState, GameState, RunningState},
};

#[cfg(feature = "dev_tools")]
type StepperMarker<'w> = Option<Res<'w, crate::dev::procgen_stepper::ProcgenStepperActive>>;

crate::support_item! {
    /// Live `State` resources a game command's facts are sampled from.
    #[derive(SystemParam)]
    struct GameFactsParam<'w> {
        app:         Res<'w, State<AppState>>,
        running:     Option<Res<'w, State<RunningState>>>,
        game:        Option<Res<'w, State<GameState>>>,
        battlescape: Option<Res<'w, State<BattleScapeState>>>,
        aftermath:   Option<Res<'w, State<AfterMathState>>>,
        battle:      Option<Res<'w, BattleInProgress>>,
        #[cfg(feature = "dev_tools")]
        stepper:     StepperMarker<'w>,
    }
}

impl GameFactsParam<'_> {
    crate::support_item! {
        /// Read the host's live phase into a facts value.
        #[must_use]
        fn sample(&self) -> GameFacts {
            #[cfg(feature = "dev_tools")]
            let stepper = match self.stepper {
                Some(_) => StepperActivity::Stepping,
                None => StepperActivity::NotStepping,
            };
            #[cfg(not(feature = "dev_tools"))]
            let stepper = StepperActivity::NotStepping;

            GameFacts::new(
                AppPhaseNet::new(
                    LifecyclePhaseNet::from_state(self.app.get()),
                    self.running
                        .as_ref()
                        .map(|state| RunningPhaseNet::from_state(*state.get())),
                    self.game
                        .as_ref()
                        .map(|state| GamePhaseNet::from_state(*state.get())),
                    self.battlescape
                        .as_ref()
                        .map(|state| BattleScapePhaseNet::from_state(*state.get())),
                    self.aftermath
                        .as_ref()
                        .map(|state| AfterMathPhaseNet::from_state(*state.get())),
                ),
                self.battle_model(),
                stepper,
            )
        }
    }

    const fn battle_model(&self) -> BattleModel {
        match self.battle {
            Some(_) => BattleModel::Present,
            None => BattleModel::Absent,
        }
    }
}
