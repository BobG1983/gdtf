use bevy::prelude::*;

use crate::states::RunningState;

#[derive(SubStates, Default, Debug, Clone, Copy, Eq, PartialEq, Hash)]
#[source(RunningState = RunningState::Game)]
pub(crate) enum GameState {
    #[default]
    Setup, // Pre-Game - Selecting Gang, Setting up the Hive, etc.
    HiveScape,   // Strategic Layer
    BattleScape, // Tactical Layer
}
