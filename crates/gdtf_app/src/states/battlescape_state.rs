use bevy::prelude::*;

use crate::states::GameState;

#[derive(SubStates, Default, Debug, Clone, Copy, Eq, PartialEq, Hash)]
#[source(GameState = GameState::BattleScape)]
pub(crate) enum BattleScapeState {
    #[default]
    Generation, // Pre-Game - Generating the BattleScape, etc.
    AnimateIn,     // Animating the BattleScape coming in, etc.
    BattleRunning, // The actual BattleScape - Tactical Layer
    AnimateOut,    // Animating the BattleScape going out, etc.
    AfterMath,     // Post-Game - Show Results, etc.
}
