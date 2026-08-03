use bevy::prelude::*;

use crate::states::BattleScapeState;

crate::support_item! {
        #[derive(SubStates, Default, Debug, Clone, Copy, Eq, PartialEq, Hash)]
    #[source(BattleScapeState = BattleScapeState::AfterMath)]
    enum AfterMathState {
                #[default]
        AnimateIn,
                DisplayAftermath,
                AnimateOut,
    }
}
