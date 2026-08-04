//! Per-turn DOT damage and expiry.

use bevy::prelude::{Entity, Message, MessageWriter, Query};

use crate::{
    effects::on_death::OnDeathOccurred,
    ganger::{Hp, LifeState, Position},
    metric::CellLevel,
    weapon::{Dot, DotDamage},
};

type DotRow = (
    Entity,
    &'static mut Hp,
    &'static mut LifeState,
    &'static mut Dot,
    &'static Position,
);

/// One tick of DOT damage was applied.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DotTicked {
    /// Ganger entity.
    pub ganger: Entity,
    /// Cell where the tick happened.
    pub at:     CellLevel,
    /// Damage dealt this tick.
    pub amount: DotDamage,
}

impl DotTicked {
    /// Build the message.
    #[must_use]
    pub const fn new(ganger: Entity, at: CellLevel, amount: DotDamage) -> Self {
        Self { ganger, at, amount }
    }
}

/// Drain HP from active DOTs and remove expired ones.
pub fn tick_dot(
    mut q: Query<DotRow>,
    mut writer: MessageWriter<DotTicked>,
    mut deaths: MessageWriter<OnDeathOccurred>,
    mut commands: bevy::prelude::Commands,
) {
    for (entity, mut hp, mut life, mut dot, position) in &mut q {
        if *life == LifeState::Dead {
            continue;
        }

        let amount = dot.per_turn_damage;
        *hp = Hp::new(hp.saturating_sub(*amount));
        writer.write(DotTicked::new(entity, **position, amount));

        if *hp == Hp::new(0) {
            *life = LifeState::Dead;
            deaths.write(OnDeathOccurred::new(entity, **position));
        }

        match dot.remaining_turns.decremented() {
            Some(next) if *life != LifeState::Dead => dot.remaining_turns = next,
            _ => {
                commands.entity(entity).remove::<Dot>();
            }
        }
    }
}
