use bevy::prelude::*;

use crate::states::BattleScapeState;

#[derive(SubStates, Default, Debug, Clone, Copy, Eq, PartialEq, Hash)]
#[source(BattleScapeState = BattleScapeState::AfterMath)]
pub(crate) enum AfterMathState {
    #[default]
    AnimateIn, // Animating the Aftermath coming in, etc.
    DisplayAftermath, // Displaying the Aftermath - Results, etc.
    AnimateOut,       // Animating the Aftermath going out, etc.
}
