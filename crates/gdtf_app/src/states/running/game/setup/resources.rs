use bevy::prelude::*;

/// The `Game::Setup` scene's completion marker. `Default` derives the unit value
/// the scaffold's generic marker insert seeds (GTW-575).
#[derive(Resource, Default)]
pub(in crate::states::running::game::setup) struct SetupComplete;
