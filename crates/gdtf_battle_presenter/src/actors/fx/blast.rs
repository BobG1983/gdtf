use bevy::{
    ecs::{message::MessageReader, template::template},
    prelude::*,
    scene::{CommandsSceneExt, bsn},
};
use gdtf_battle_sim::acts::ThrowResolved;

use super::projectile::PendingImpact;
use crate::{cell_to_world, playback::Played};

pub fn read_throw_resolved(
    mut commands: Commands,
    mut resolved: MessageReader<Played<ThrowResolved>>,
) {
    for msg in resolved.read() {
        let (cell, level) = msg.at.split();
        let at = cell_to_world(cell, level);
        let pending = PendingImpact::for_blast(at, msg.damage);
        commands.spawn_scene(bsn! { template(move |_| Ok(pending.clone())) });
    }
}
