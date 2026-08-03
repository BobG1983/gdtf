use bevy::prelude::*;

use crate::states::RunningState;

crate::support_item! {
        #[derive(SubStates, Default, Debug, Clone, Copy, Eq, PartialEq, Hash)]
    #[source(RunningState = RunningState::Game)]
    enum GameState {
                #[default]
        Setup,
                HiveScape,
                BattleScape,
    }
}
