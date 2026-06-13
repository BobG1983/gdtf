use bevy::prelude::*;

use crate::states::BattleScapeState;

crate::support_item! {
    /// Sub-state of [`BattleScapeState::AfterMath`]: phase within the aftermath.
    #[derive(SubStates, Default, Debug, Clone, Copy, Eq, PartialEq, Hash)]
    #[source(BattleScapeState = BattleScapeState::AfterMath)]
    enum AfterMathState {
        /// Animating the aftermath coming in.
        #[default]
        AnimateIn,
        /// Displaying the aftermath results.
        DisplayAftermath,
        /// Animating the aftermath going out.
        AnimateOut,
    }
}
