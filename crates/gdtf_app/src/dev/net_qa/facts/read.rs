//! [`GameFactsParam`] — the ONE read of the world that produces a frame's [`GameFacts`]
//! (GTW-942).

use bevy::{ecs::system::SystemParam, prelude::*};

use super::GameFacts;
use crate::{
    dev::net_qa::wire::{
        AfterMathPhaseNet, AppPhaseNet, BattleScapePhaseNet, GamePhaseNet, LifecyclePhaseNet,
        RunningPhaseNet,
    },
    states::{AfterMathState, AppState, BattleScapeState, GameState, RunningState},
};

/// The five state resources a frame's facts are read from.
///
/// Every sub-state is `Option<Res<_>>` and NOT a `run_if` gate, because a `SubStates`
/// resource is absent whenever its parent does not hold the sourcing variant
/// (`bevy-traps.md` #1). A router that took them as plain `Res` would fail parameter
/// validation on the menu, which is precisely where `app.phase` has to be answerable.
#[derive(SystemParam)]
pub(crate) struct GameFactsParam<'w> {
    /// The top-level lifecycle state — always present once the app is built.
    app:         Res<'w, State<AppState>>,
    /// The running screen, present only under [`AppState::Running`].
    running:     Option<Res<'w, State<RunningState>>>,
    /// The game layer, present only under [`RunningState::Game`].
    game:        Option<Res<'w, State<GameState>>>,
    /// The battle-layer phase, present only under [`GameState::BattleScape`].
    battlescape: Option<Res<'w, State<BattleScapeState>>>,
    /// The aftermath phase, present only under [`BattleScapeState::AfterMath`].
    aftermath:   Option<Res<'w, State<AfterMathState>>>,
}

impl GameFactsParam<'_> {
    /// Read the frame's facts — called ONCE per frame, by the router, before it drains.
    ///
    /// Sampling once rather than per request is what makes every request in one drain see
    /// the same world: two calls in a single frame that disagreed about whether a battle
    /// was running would be answered from two different worlds that never existed.
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
