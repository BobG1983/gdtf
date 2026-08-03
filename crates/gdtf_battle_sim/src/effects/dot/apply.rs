//! Apply DOT profiles to targets from messages.

use bevy::prelude::{Commands, Entity, Message, MessageReader, MessageWriter, Query};

use crate::weapon::{Dot, DotDamage};

/// Request to attach or refresh a DOT on a target.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DotApplied {
    /// Target entity.
    pub target: Entity,
    /// DOT profile to apply.
    pub dot: Dot,
}

impl DotApplied {
    /// Build the message.
    #[must_use]
    pub const fn new(target: Entity, dot: Dot) -> Self {
        Self { target, dot }
    }
}

/// A DOT was newly attached to a ganger.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DotAfflicted {
    /// Ganger entity.
    pub ganger: Entity,
    /// Damage per turn.
    pub per_turn: DotDamage,
}

impl DotAfflicted {
    /// Build the message.
    #[must_use]
    pub const fn new(ganger: Entity, per_turn: DotDamage) -> Self {
        Self { ganger, per_turn }
    }
}

/// Attach or refresh DOTs from [`DotApplied`] messages.
pub fn apply_dot(
    mut applied: MessageReader<DotApplied>,
    mut existing: Query<&mut Dot>,
    mut commands: Commands,
    mut afflicted: MessageWriter<DotAfflicted>,
) {
    let mut attached_this_tick: bevy::platform::collections::HashSet<Entity> =
        bevy::platform::collections::HashSet::default();
    for message in applied.read() {
        let target = message.target;
        let Ok(mut entity) = commands.get_entity(target) else {
            continue;
        };
        if let Ok(mut dot) = existing.get_mut(target) {
            dot.refresh_from(crate::weapon::DotProfile::new(
                message.dot.per_turn_damage,
                message.dot.damage_type,
                message.dot.remaining_turns,
            ));
        } else if attached_this_tick.contains(&target) {
            entity.insert(message.dot);
        } else {
            entity.insert(message.dot);
            afflicted.write(DotAfflicted::new(target, message.dot.per_turn_damage));
            attached_this_tick.insert(target);
        }
    }
}
