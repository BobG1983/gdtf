use bevy::{ecs::system::SystemParam, prelude::*};

use super::GameFacts;
use crate::{
    dev::net_qa::wire::{
        AfterMathPhaseNet, AppPhaseNet, BattleScapePhaseNet, GamePhaseNet, LifecyclePhaseNet,
        RunningPhaseNet,
    },
    states::{AfterMathState, AppState, BattleScapeState, GameState, RunningState},
};

#[derive(SystemParam)]
pub(crate) struct GameFactsParam<'w> {
    app:         Res<'w, State<AppState>>,
    running:     Option<Res<'w, State<RunningState>>>,
    game:        Option<Res<'w, State<GameState>>>,
    battlescape: Option<Res<'w, State<BattleScapeState>>>,
    aftermath:   Option<Res<'w, State<AfterMathState>>>,
}

impl GameFactsParam<'_> {
    pub(crate) fn sample(&self) -> GameFacts {
        GameFacts::new(AppPhaseNet::new(
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
        ))
    }
}
