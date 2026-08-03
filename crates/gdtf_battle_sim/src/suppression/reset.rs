use bevy::prelude::{Commands, Entity, MessageReader, Query};

use crate::{
    ganger::{Faction, Suppressed},
    turn::TurnStarted,
};

pub fn reset_suppression(
    mut turns: MessageReader<TurnStarted>,
    suppressed: Query<(Entity, &Faction), bevy::prelude::With<Suppressed>>,
    mut commands: Commands,
) {
    for turn in turns.read() {
        let now_active = turn.now_active;
        for (entity, faction) in &suppressed {
            if *faction == now_active {
                commands.entity(entity).remove::<Suppressed>();
            }
        }
    }
}
