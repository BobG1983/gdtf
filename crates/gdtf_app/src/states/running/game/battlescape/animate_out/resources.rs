use bevy::prelude::*;

/// The `Game::BattleScape::AnimateOut` scene's completion marker. `Default`
/// derives the unit value the scaffold's generic marker insert seeds (GTW-575).
#[derive(Resource, Default)]
pub(in crate::states::running::game::battlescape::animate_out) struct BattleAnimateOutComplete;
