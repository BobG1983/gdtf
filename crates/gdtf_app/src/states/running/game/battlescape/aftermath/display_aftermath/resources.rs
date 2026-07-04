use bevy::prelude::*;

/// The `Game::BattleScape::AfterMath::DisplayAftermath` scene's completion
/// marker. `Default` derives the unit value the scaffold's generic marker
/// insert seeds (GTW-575).
#[derive(Resource, Default)]
pub(in crate::states::running::game::battlescape::aftermath::display_aftermath) struct DisplayAftermathComplete;
