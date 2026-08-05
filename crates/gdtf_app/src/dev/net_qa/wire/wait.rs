//! Conditions a `wait` call holds out for, and the phase target one of them names.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

use super::phase::{
    AfterMathPhaseNet, BattleScapePhaseNet, GamePhaseNet, LifecyclePhaseNet, RunningPhaseNet,
};

crate::support_item! {
    /// How many act-log entries a wait is holding out for.
    #[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(transparent)]
    pub(crate) struct ActCountNet(usize);
}

#[cfg(feature = "headless_test")]
impl ActCountNet {
    /// Build from an entry count. Callers off the wire decode one instead.
    #[must_use]
    pub const fn new(entries: usize) -> Self {
        Self(entries)
    }
}

crate::support_item! {
    /// A phase to wait for. Every level left `None` matches anything.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
    #[serde(default, deny_unknown_fields)]
    pub(crate) struct AppPhaseTargetNet {
        app:         Option<LifecyclePhaseNet>,
        running:     Option<RunningPhaseNet>,
        game:        Option<GamePhaseNet>,
        battlescape: Option<BattleScapePhaseNet>,
        aftermath:   Option<AfterMathPhaseNet>,
    }
}

#[cfg(feature = "headless_test")]
impl AppPhaseTargetNet {
    /// Build a target from the levels it pins. Callers off the wire decode one instead.
    #[must_use]
    pub const fn new(
        app: Option<LifecyclePhaseNet>,
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

impl AppPhaseTargetNet {
    crate::support_item! {
        /// Lifecycle phase this target pins, if any.
        #[must_use]
        const fn app(self) -> Option<LifecyclePhaseNet> {
            self.app
        }
    }

    crate::support_item! {
        /// Running screen this target pins, if any.
        #[must_use]
        const fn running(self) -> Option<RunningPhaseNet> {
            self.running
        }
    }

    crate::support_item! {
        /// Game layer this target pins, if any.
        #[must_use]
        const fn game(self) -> Option<GamePhaseNet> {
            self.game
        }
    }

    crate::support_item! {
        /// Battle phase this target pins, if any.
        #[must_use]
        const fn battlescape(self) -> Option<BattleScapePhaseNet> {
            self.battlescape
        }
    }

    crate::support_item! {
        /// Aftermath phase this target pins, if any.
        #[must_use]
        const fn aftermath(self) -> Option<AfterMathPhaseNet> {
            self.aftermath
        }
    }
}

crate::support_item! {
    /// What a `wait` call is holding out for.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub(crate) enum WaitConditionNet {
        /// The screen has caught up with the act log.
        CaughtUp,
        /// The live phase matches every level the target names.
        Phase(AppPhaseTargetNet),
        /// The act log holds at least this many entries.
        LogAtLeast(ActCountNet),
        /// No ganger is part-way through a walk.
        WalkComplete,
        /// A turn hand-off happened after the call was admitted.
        TurnChanged,
        /// The battle-running phase has finished.
        BattleDecided,
        /// The situation has finished generating.
        GenerationComplete,
    }
}
