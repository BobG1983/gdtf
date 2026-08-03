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

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DotTicked {
        pub ganger: Entity,
        pub at:     CellLevel,
        pub amount: DotDamage,
}

impl DotTicked {
        #[must_use]
    pub const fn new(ganger: Entity, at: CellLevel, amount: DotDamage) -> Self {
        Self { ganger, at, amount }
    }
}

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
