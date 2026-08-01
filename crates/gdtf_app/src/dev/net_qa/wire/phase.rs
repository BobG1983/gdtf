//! [`AppPhaseNet`] and the five state mirrors it is built from (GTW-942).
//!
//! # Why the mirrors are minted HERE
//!
//! The game's five state enums live in [`crate::states`] and are declared through
//! `crate::support_item!`. Deriving [`JsonSchema`] on them would mean putting a
//! `schemars` derive inside those macro invocations — five of them — which drags a
//! schema-derivation dependency into the state machine itself, for the benefit of one
//! dev-only QA command. So `crate::states` is NOT touched: the mirrors are declared here,
//! next to the command that publishes them, and a `from_state` constructor maps each one
//! across. A state variant added over there breaks the wildcard-free `match` here until
//! this mirror gains its own arm.
//!
//! # Why five levels, not one
//!
//! The old app-flow snapshot reported only the TOP-level [`AppState`], so "which battle
//! phase is the app in" — the question every QA pass actually asks — was unanswerable over
//! the wire. The four sub-states are nested: each is
//! present only while its parent holds the variant that sources it, which is exactly what
//! the `Option`s below say.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::states::{AfterMathState, AppState, BattleScapeState, GameState, RunningState};

/// The wire mirror of [`AppState`] — the app's top-level lifecycle phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub(crate) enum LifecyclePhaseNet {
    /// One-shot bootstrap before any scene runs.
    Init,
    /// Asset / scene loading hand-off.
    Load,
    /// Intro / splash scene.
    Intro,
    /// The running game.
    Running,
    /// Final teardown before exit.
    Teardown,
}

impl LifecyclePhaseNet {
    /// Mirror the live [`AppState`].
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

/// The wire mirror of [`RunningState`] — which top-level running screen is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub(crate) enum RunningPhaseNet {
    /// Main menu.
    Menu,
    /// In-game (setup, hive layer, battle layer).
    Game,
    /// Options screen.
    Options,
    /// Quit confirmation / shutdown.
    Quit,
}

impl RunningPhaseNet {
    /// Mirror the live [`RunningState`].
    pub(crate) const fn from_state(state: RunningState) -> Self {
        match state {
            RunningState::Menu => Self::Menu,
            RunningState::Game => Self::Game,
            RunningState::Options => Self::Options,
            RunningState::Quit => Self::Quit,
        }
    }
}

/// The wire mirror of [`GameState`] — which game layer is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub(crate) enum GamePhaseNet {
    /// Pre-game: selecting gang, setting up the hive.
    Setup,
    /// Strategic (hive) layer.
    HiveScape,
    /// Tactical (battle) layer.
    BattleScape,
}

impl GamePhaseNet {
    /// Mirror the live [`GameState`].
    pub(crate) const fn from_state(state: GameState) -> Self {
        match state {
            GameState::Setup => Self::Setup,
            GameState::HiveScape => Self::HiveScape,
            GameState::BattleScape => Self::BattleScape,
        }
    }
}

/// The wire mirror of [`BattleScapeState`] — the phase within the battle layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub(crate) enum BattleScapePhaseNet {
    /// Pre-game: generating the battlescape.
    Generation,
    /// Animating the battlescape coming in.
    AnimateIn,
    /// The tactical layer running.
    BattleRunning,
    /// Animating the battlescape going out.
    AnimateOut,
    /// Post-game: showing results.
    AfterMath,
}

impl BattleScapePhaseNet {
    /// Mirror the live [`BattleScapeState`].
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

/// The wire mirror of [`AfterMathState`] — the phase within the aftermath.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub(crate) enum AfterMathPhaseNet {
    /// Animating the aftermath coming in.
    AnimateIn,
    /// Displaying the aftermath results.
    DisplayAftermath,
    /// Animating the aftermath going out.
    AnimateOut,
}

impl AfterMathPhaseNet {
    /// Mirror the live [`AfterMathState`].
    pub(crate) const fn from_state(state: AfterMathState) -> Self {
        match state {
            AfterMathState::AnimateIn => Self::AnimateIn,
            AfterMathState::DisplayAftermath => Self::DisplayAftermath,
            AfterMathState::AnimateOut => Self::AnimateOut,
        }
    }
}

/// Where the app is, at every level of its state machine at once.
///
/// The top level is always present; each nested level is present only while its parent
/// holds the variant that sources it, so `None` is a real answer rather than a missing
/// value: "the app is not in a running game at all" and "the app is in a running game
/// whose layer I could not read" must not look the same to a client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub(crate) struct AppPhaseNet {
    /// The top-level lifecycle phase — always present.
    app:         LifecyclePhaseNet,
    /// The running screen, present only under [`LifecyclePhaseNet::Running`].
    running:     Option<RunningPhaseNet>,
    /// The game layer, present only under [`RunningPhaseNet::Game`].
    game:        Option<GamePhaseNet>,
    /// The battle-layer phase, present only under [`GamePhaseNet::BattleScape`].
    battlescape: Option<BattleScapePhaseNet>,
    /// The aftermath phase, present only under [`BattleScapePhaseNet::AfterMath`].
    aftermath:   Option<AfterMathPhaseNet>,
}

impl AppPhaseNet {
    /// Build the five-level phase from the levels that are live right now.
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
