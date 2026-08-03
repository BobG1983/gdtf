use bevy::prelude::{MessageReader, Res};

use crate::{battle::PlayerFaction, turn::TurnStarted};

#[must_use]
pub fn enemy_phase_started(
    mut turns: MessageReader<TurnStarted>,
    player: Option<Res<PlayerFaction>>,
) -> bool {
    let Some(player) = player else {
        return false;
    };
    let player = **player;
    turns.read().any(|started| started.now_active != player)
}
