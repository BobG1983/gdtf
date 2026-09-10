use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_presenter::playback::PlaybackGate;
use gdtf_battle_sim::{battle::PlayerFaction, prelude::BattleInProgress, turn::ActiveFaction};

#[cfg(feature = "dev_tools")]
use super::StepperActivity;
use super::{BattleModel, GameFacts, PlaybackCatchUp, TurnOwner};
use crate::{
    dev::mcp::wire::{
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
        active:      Option<Res<'w, ActiveFaction>>,
        player:      Option<Res<'w, PlayerFaction>>,
        gate:        PlaybackGate<'w>,
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
                #[cfg(feature = "dev_tools")]
                stepper,
                self.catch_up(),
                self.turn_owner(),
            )
        }
    }

    const fn battle_model(&self) -> BattleModel {
        match self.battle {
            Some(_) => BattleModel::Present,
            None => BattleModel::Absent,
        }
    }

    fn turn_owner(&self) -> TurnOwner {
        let acting = self.active.as_ref().map(|active| ***active);
        let commanded = self.player.as_ref().map(|player| ***player);
        if acting.is_some() && acting == commanded {
            TurnOwner::Player
        } else {
            TurnOwner::OtherFaction
        }
    }

    fn catch_up(&self) -> PlaybackCatchUp {
        if self.gate.is_open() {
            PlaybackCatchUp::CaughtUp
        } else {
            PlaybackCatchUp::Behind
        }
    }
}
