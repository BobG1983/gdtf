use bevy::prelude::*;

use crate::states::AppState;

crate::support_item! {
        #[derive(SubStates, Default, Debug, Clone, Copy, Eq, PartialEq, Hash)]
    #[source(AppState = AppState::Running)]
    enum RunningState {
                #[default]
        Menu,
                Game,
                Options,
                Quit,
    }
}
