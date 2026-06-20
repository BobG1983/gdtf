use bevy::prelude::*;

use crate::states::GameState;

crate::support_item! {
    /// Sub-state of [`GameState::BattleScape`]: phase within the battle layer.
    #[derive(SubStates, Default, Debug, Clone, Copy, Eq, PartialEq, Hash)]
    #[source(GameState = GameState::BattleScape)]
    enum BattleScapeState {
        /// Pre-game: generating the battlescape.
        #[default]
        Generation,
        /// Animating the battlescape coming in.
        AnimateIn,
        /// The actual battlescape running (tactical layer).
        BattleRunning,
        /// Animating the battlescape going out.
        AnimateOut,
        /// Post-game: showing results.
        AfterMath,
    }
}
