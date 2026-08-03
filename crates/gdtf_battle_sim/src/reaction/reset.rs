use bevy::prelude::{MessageReader, Query};

use crate::tuning::ReactionsUsed;

pub fn reset_reactions_used(
    mut turns: MessageReader<crate::turn::TurnStarted>,
    mut used: Query<&mut ReactionsUsed>,
) {
    let mut crossed = false;
    for _turn in turns.read() {
        crossed = true;
    }
    if !crossed {
        return;
    }
    for mut counter in &mut used {
        counter.reset();
    }
}
