use bevy::prelude::*;

use crate::states::RunningState;

crate::support_item! {
    /// Sub-state of [`RunningState::Game`]: which game layer is active.
    #[derive(SubStates, Default, Debug, Clone, Copy, Eq, PartialEq, Hash)]
    #[source(RunningState = RunningState::Game)]
    enum GameState {
        /// Pre-game: selecting gang, setting up the hive, etc.
        #[default]
        Setup,
        /// Strategic (hive) layer.
        HiveScape,
        /// Tactical (battle) layer.
        BattleScape,
    }
}
