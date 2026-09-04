//! Sub-states while the app is in [`BattleScapeState::AfterMath`].

use bevy::prelude::*;

use crate::states::BattleScapeState;

crate::support_item! {
    /// Phases of the post-battle screen.
    #[derive(SubStates, Default, Debug, Clone, Copy, Eq, PartialEq, Hash)]
    #[source(BattleScapeState = BattleScapeState::AfterMath)]
    enum AfterMathState {
        /// Animate into the summary.
        #[default]
        AnimateIn,
        /// Show the aftermath panel.
        DisplayAftermath,
        /// Animate out.
        AnimateOut,
    }
}
