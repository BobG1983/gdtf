use bevy::prelude::*;

use crate::states::GameState;

crate::support_item! {
        #[derive(SubStates, Default, Debug, Clone, Copy, Eq, PartialEq, Hash)]
    #[source(GameState = GameState::BattleScape)]
    enum BattleScapeState {
                #[default]
        Generation,
                AnimateIn,
                BattleRunning,
                AnimateOut,
                AfterMath,
    }
}
