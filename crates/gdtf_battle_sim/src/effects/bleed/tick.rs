use bevy::prelude::{Commands, Component, Entity, Message, MessageWriter, Query, Res};

use crate::{
    effects::on_death::OnDeathOccurred,
    ganger::{Hp, LifeState, Position, Wounds},
    injuries::BleedAfflicted,
    tuning::CombatTuning,
};

type BleedRow = (
    Entity,
    Option<&'static mut Hp>,
    &'static mut Wounds,
    &'static mut LifeState,
    Option<&'static BleedingOut>,
    Option<&'static BleedAfflicted>,
    Option<&'static Position>,
    Option<&'static BleedOngoing>,
);

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct BleedOngoing;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct BleedingOut;

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BleedStarted {
        pub ganger: Entity,
}

impl BleedStarted {
        #[must_use]
    pub const fn new(ganger: Entity) -> Self {
        Self { ganger }
    }
}

/// A buffered Bevy **message** (`#[derive(Message)]`), mirroring
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bleeding {
        pub ganger: Entity,
}

impl Bleeding {
            #[must_use]
    pub const fn new(ganger: Entity) -> Self {
        Self { ganger }
    }
}

pub fn tick_bleed(
    mut q: Query<BleedRow>,
    tuning: Res<CombatTuning>,
    mut writer: MessageWriter<Bleeding>,
    mut deaths: MessageWriter<OnDeathOccurred>,
    mut started: MessageWriter<BleedStarted>,
    mut commands: Commands,
) {
    let rate = *tuning.bleed_rate;
    for (entity, hp, mut wounds, mut life, bleeding_out, bleed, position, ongoing) in &mut q {
        if *life == LifeState::Dead {
            if ongoing.is_some() {
                commands.entity(entity).remove::<BleedOngoing>();
            }
            continue;
        }

        let mut drained = false;

        let injury_bleed = bleed.map_or(0u16, |b| **b);
        if let Some(mut hp) = hp
            && injury_bleed > 0
        {
            *hp = Hp::new(hp.saturating_sub(injury_bleed));
            writer.write(Bleeding::new(entity));
            drained = true;
            if *hp == Hp::new(0) && *life == LifeState::Alive {
                *life = LifeState::Downed;
                commands.entity(entity).insert(BleedingOut);
            }
        }

        if bleeding_out.is_some() {
            *wounds = Wounds::new(wounds.saturating_sub(rate));
            writer.write(Bleeding::new(entity));
            drained = true;

            if *wounds == Wounds::new(0) {
                *life = LifeState::Dead;
                if let Some(position) = position {
                    deaths.write(OnDeathOccurred::new(entity, **position));
                }
            }
        }

        match (drained, ongoing.is_some()) {
            (true, false) => {
                started.write(BleedStarted::new(entity));
                commands.entity(entity).insert(BleedOngoing);
            }
            (false, true) => {
                commands.entity(entity).remove::<BleedOngoing>();
            }
            _ => {}
        }
    }
}
