//! Sub-states while the app is in [`RunningState::Game`].

use bevy::prelude::*;

use crate::states::RunningState;

crate::support_item! {
    /// Modes available under Game.
    #[derive(SubStates, Default, Debug, Clone, Copy, Eq, PartialEq, Hash)]
    #[source(RunningState = RunningState::Game)]
    enum GameState {
        /// Pre-mission setup.
        #[default]
        Setup,
        /// Campaign / hive map layer.
        HiveScape,
        /// Tactical battle.
        BattleScape,
    }
}
