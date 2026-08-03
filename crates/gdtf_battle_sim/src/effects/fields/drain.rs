//! Field drain: flat HP damage to occupants.

use bevy::prelude::{Deref, Entity};
use serde::Deserialize;

use super::{ApplyFieldEffect, OccupantDrain};
use crate::{
    effects::{fields::FieldTicked, on_death::OnDeathOccurred},
    ganger::{Hp, LifeState},
    metric::CellLevel,
};

/// Flat HP damage dealt by a field tick.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct FieldDamage(u16);

impl FieldDamage {
    /// Wrap a damage amount.
    #[must_use]
    pub const fn new(damage: u16) -> Self {
        Self(damage)
    }
}

/// Applies flat drain damage to an occupant.
pub struct ApplyDrain {
    damage: FieldDamage,
}

impl ApplyDrain {
    /// Build the applicator.
    #[must_use]
    pub const fn new(damage: FieldDamage) -> Self {
        Self { damage }
    }
}

impl ApplyFieldEffect for ApplyDrain {
    fn drain_occupant(
        &self,
        at: CellLevel,
        occupant: Entity,
        drain: &mut OccupantDrain<'_, '_, '_, '_, '_>,
    ) {
        **drain.hp = Hp::new(drain.hp.saturating_sub(*self.damage));
        drain
            .ticks
            .write(FieldTicked::new(occupant, at, self.damage));
        if **drain.hp == Hp::new(0) {
            **drain.life = LifeState::Dead;
            drain.deaths.write(OnDeathOccurred::new(occupant, at));
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::{
        ecs::system::SystemState,
        prelude::{Entity, MessageWriter, Messages, Query, World},
    };

    use super::{ApplyDrain, FieldDamage};
    use crate::{
        effects::{
            fields::{ApplyFieldEffect, FieldTicked, OccupantDrain},
            on_death::OnDeathOccurred,
        },
        ganger::{Hp, LifeState},
        metric::{Cell, CellLevel, Level},
    };

    fn ground(x: i32, y: i32) -> CellLevel {
        CellLevel::new(Cell::new(x, y), Level::new(0))
    }

    type DrainParams = SystemState<(
        Query<'static, 'static, (&'static mut Hp, &'static mut LifeState)>,
        MessageWriter<'static, FieldTicked>,
        MessageWriter<'static, OnDeathOccurred>,
    )>;

    fn drained(damage: u16, start_hp: u16) -> (u16, LifeState, usize, usize) {
        let mut world = World::new();
        world.init_resource::<Messages<FieldTicked>>();
        world.init_resource::<Messages<OnDeathOccurred>>();
        let occupant = world.spawn((Hp::new(start_hp), LifeState::Alive)).id();
        let mut state: DrainParams = SystemState::new(&mut world);
        run_drain(&mut state, &mut world, occupant, damage);
        let hp = world.get::<Hp>(occupant).map_or(0, |h| **h);
        let life = world
            .get::<LifeState>(occupant)
            .copied()
            .unwrap_or(LifeState::Alive);
        let ticks = world.resource::<Messages<FieldTicked>>().len();
        let deaths = world.resource::<Messages<OnDeathOccurred>>().len();
        (hp, life, ticks, deaths)
    }

    fn run_drain(state: &mut DrainParams, world: &mut World, occupant: Entity, damage: u16) {
        let Ok((mut occupants, mut ticks, mut deaths)) = state.get_mut(world) else {
            unreachable!("the params validate: both message buffers are initialized");
        };
        let Ok((mut hp, mut life)) = occupants.get_mut(occupant) else {
            unreachable!("the occupant was just spawned with Hp + LifeState");
        };
        let mut drain = OccupantDrain {
            hp: &mut hp,
            life: &mut life,
            ticks: &mut ticks,
            deaths: &mut deaths,
        };
        ApplyDrain::new(FieldDamage::new(damage)).drain_occupant(
            ground(4, 4),
            occupant,
            &mut drain,
        );
    }

    #[test]
    fn a_non_lethal_drain_eats_the_flat_amount_and_signals_one_tick() {
        let (hp, life, ticks, deaths) = drained(3, 10);
        assert_eq!(hp, 7, "the drain is a flat direct subtraction");
        assert_eq!(life, LifeState::Alive, "a non-lethal drain leaves Alive");
        assert_eq!(ticks, 1, "one FieldTicked per draining round");
        assert_eq!(deaths, 0, "a non-lethal drain emits no death signal");
    }

    #[test]
    fn a_lethal_drain_floors_at_zero_kills_and_signals_the_death() {
        let (hp, life, ticks, deaths) = drained(10, 8);
        assert_eq!(hp, 0, "the drain saturates — no underflow");
        assert_eq!(
            life,
            LifeState::Dead,
            "an emptied pool KILLS (DOT precedent)"
        );
        assert_eq!(ticks, 1, "the lethal tick still signals FieldTicked");
        assert_eq!(deaths, 1, "the lethal tick emits one OnDeathOccurred");
    }
}
