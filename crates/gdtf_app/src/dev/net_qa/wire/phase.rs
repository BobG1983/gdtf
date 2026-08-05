//! Lifecycle, screen, layer, battle, and aftermath phases on the wire.

use serde::{Deserialize, Serialize};

use crate::states::{AfterMathState, AppState, BattleScapeState, GameState, RunningState};

crate::support_item! {
    /// Top-level lifecycle phase.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub(crate) enum LifecyclePhaseNet {
        /// Startup bootstrap.
        Init,
        /// Loading assets and tables.
        Load,
        /// Intro / splash.
        Intro,
        /// Main interactive loop.
        Running,
        /// Shutdown cleanup.
        Teardown,
    }
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

crate::support_item! {
    /// Screen inside the running lifecycle phase.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub(crate) enum RunningPhaseNet {
        /// Main menu.
        Menu,
        /// In game.
        Game,
        /// Options screen.
        Options,
        /// Quitting.
        Quit,
    }
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

crate::support_item! {
    /// Layer inside the game screen.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub(crate) enum GamePhaseNet {
        /// Game setup.
        Setup,
        /// Hive map.
        HiveScape,
        /// Battle map.
        BattleScape,
    }
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

crate::support_item! {
    /// Phase inside the battle map.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub(crate) enum BattleScapePhaseNet {
        /// Generating the map.
        Generation,
        /// Animating in.
        AnimateIn,
        /// Battle running.
        BattleRunning,
        /// Animating out.
        AnimateOut,
        /// Aftermath.
        AfterMath,
    }
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

crate::support_item! {
    /// Phase inside the aftermath screen.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub(crate) enum AfterMathPhaseNet {
        /// Animating in.
        AnimateIn,
        /// Showing the aftermath.
        DisplayAftermath,
        /// Animating out.
        AnimateOut,
    }
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

crate::support_item! {
    /// Where the app is at every level of its state machine.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct AppPhaseNet {
        app:         LifecyclePhaseNet,
        running:     Option<RunningPhaseNet>,
        game:        Option<GamePhaseNet>,
        battlescape: Option<BattleScapePhaseNet>,
        aftermath:   Option<AfterMathPhaseNet>,
    }
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

    crate::support_item! {
        /// The lifecycle phase, which is always live.
        #[must_use]
        pub(crate) const fn app(self) -> LifecyclePhaseNet {
            self.app
        }
    }

    crate::support_item! {
        /// The running screen, where the lifecycle phase is running.
        #[must_use]
        pub(crate) const fn running(self) -> Option<RunningPhaseNet> {
            self.running
        }
    }

    crate::support_item! {
        /// The game layer, where the running screen is the game.
        #[must_use]
        pub(crate) const fn game(self) -> Option<GamePhaseNet> {
            self.game
        }
    }

    crate::support_item! {
        /// The battle phase, where the game layer is the battle map.
        #[must_use]
        pub(crate) const fn battlescape(self) -> Option<BattleScapePhaseNet> {
            self.battlescape
        }
    }

    crate::support_item! {
        /// The aftermath phase, where the battle phase is the aftermath.
        #[must_use]
        pub(crate) const fn aftermath(self) -> Option<AfterMathPhaseNet> {
            self.aftermath
        }
    }
}
