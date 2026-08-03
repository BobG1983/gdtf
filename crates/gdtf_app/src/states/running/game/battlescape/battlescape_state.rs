//! Sub-states while the app is in [`GameState::BattleScape`].

use bevy::prelude::*;

use crate::states::GameState;

crate::support_item! {
    /// Phases of a battlescape encounter.
    #[derive(SubStates, Default, Debug, Clone, Copy, Eq, PartialEq, Hash)]
    #[source(GameState = GameState::BattleScape)]
    enum BattleScapeState {
        /// Map generation / load-in.
        #[default]
        Generation,
        /// Intro animation.
        AnimateIn,
        /// Active combat.
        BattleRunning,
        /// Outro animation.
        AnimateOut,
        /// Post-battle summary.
        AfterMath,
    }
}
