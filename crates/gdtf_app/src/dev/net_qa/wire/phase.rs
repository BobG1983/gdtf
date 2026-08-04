use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::states::{AfterMathState, AppState, BattleScapeState, GameState, RunningState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub(crate) enum LifecyclePhaseNet {
    Init,
    Load,
    Intro,
    Running,
    Teardown,
}

impl LifecyclePhaseNet {
    pub(crate) const fn from_state(state: &AppState) -> Self {
        match state {
            AppState::Init => Self::Init,
            AppState::Load => Self::Load,
            AppState::Intro => Self::Intro,
            AppState::Running => Self::Running,
            AppState::Teardown => Self::Teardown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub(crate) enum RunningPhaseNet {
    Menu,
    Game,
    Options,
    Quit,
}

impl RunningPhaseNet {
    pub(crate) const fn from_state(state: RunningState) -> Self {
        match state {
            RunningState::Menu => Self::Menu,
            RunningState::Game => Self::Game,
            RunningState::Options => Self::Options,
            RunningState::Quit => Self::Quit,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub(crate) enum GamePhaseNet {
    Setup,
    HiveScape,
    BattleScape,
}

impl GamePhaseNet {
    pub(crate) const fn from_state(state: GameState) -> Self {
        match state {
            GameState::Setup => Self::Setup,
            GameState::HiveScape => Self::HiveScape,
            GameState::BattleScape => Self::BattleScape,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub(crate) enum BattleScapePhaseNet {
    Generation,
    AnimateIn,
    BattleRunning,
    AnimateOut,
    AfterMath,
}

impl BattleScapePhaseNet {
    pub(crate) const fn from_state(state: BattleScapeState) -> Self {
        match state {
            BattleScapeState::Generation => Self::Generation,
            BattleScapeState::AnimateIn => Self::AnimateIn,
            BattleScapeState::BattleRunning => Self::BattleRunning,
            BattleScapeState::AnimateOut => Self::AnimateOut,
            BattleScapeState::AfterMath => Self::AfterMath,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub(crate) enum AfterMathPhaseNet {
    AnimateIn,
    DisplayAftermath,
    AnimateOut,
}

impl AfterMathPhaseNet {
    pub(crate) const fn from_state(state: AfterMathState) -> Self {
        match state {
            AfterMathState::AnimateIn => Self::AnimateIn,
            AfterMathState::DisplayAftermath => Self::DisplayAftermath,
            AfterMathState::AnimateOut => Self::AnimateOut,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct AppPhaseNet {
    app:         LifecyclePhaseNet,
    running:     Option<RunningPhaseNet>,
    game:        Option<GamePhaseNet>,
    battlescape: Option<BattleScapePhaseNet>,
    aftermath:   Option<AfterMathPhaseNet>,
}

impl AppPhaseNet {
    pub(crate) const fn new(
        app: LifecyclePhaseNet,
        running: Option<RunningPhaseNet>,
        game: Option<GamePhaseNet>,
        battlescape: Option<BattleScapePhaseNet>,
        aftermath: Option<AfterMathPhaseNet>,
    ) -> Self {
        Self {
            app,
            running,
            game,
            battlescape,
            aftermath,
        }
    }
}
