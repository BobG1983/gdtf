//! Per-turn field drain against occupants.

use bevy::{
    ecs::system::SystemParam,
    prelude::{Commands, Component, Entity, Message, MessageWriter, Query, Res, ResMut, With},
};

use super::{FieldDamage, FieldDef, FieldRegistry};
use crate::{
    armor::{ArmorType, Wears, WornBy},
    effects::{
        fields::{ApplyFieldEffect, FieldEffect, OccupantArmor, OccupantDrain},
        on_death::OnDeathOccurred,
    },
    ganger::{Hp, LifeState},
    metric::CellLevel,
    occupancy::OccupancyGrid,
};

/// One field tick damaged an occupant.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldTicked {
    /// Occupant entity.
    pub occupant: Entity,
    /// Cell of the field.
    pub at:       CellLevel,
    /// Damage dealt.
    pub amount:   FieldDamage,
}

impl FieldTicked {
    /// Build the message.
    #[must_use]
    pub const fn new(occupant: Entity, at: CellLevel, amount: FieldDamage) -> Self {
        Self {
            occupant,
            at,
            amount,
        }
    }
}

/// Marker: this entity is currently under field drain.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct FieldOngoing;

/// Field drain just started on an occupant.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldAfflicted {
    /// Occupant entity.
    pub occupant: Entity,
    /// Cell of the field.
    pub at:       CellLevel,
}

impl FieldAfflicted {
    /// Build the message.
    #[must_use]
    pub const fn new(occupant: Entity, at: CellLevel) -> Self {
        Self { occupant, at }
    }
}

/// Combatants a field can drain, their worn armor, and who is already draining.
#[derive(SystemParam)]
pub struct FieldOccupants<'w, 's> {
    vitals:  Query<'w, 's, (&'static mut Hp, &'static mut LifeState, &'static Wears)>,
    worn:    Query<'w, 's, &'static ArmorType, With<WornBy>>,
    ongoing: Query<'w, 's, Entity, With<FieldOngoing>>,
}

/// Messages a field tick announces.
#[derive(SystemParam)]
pub struct FieldDrainSignals<'w> {
    ticks:     MessageWriter<'w, FieldTicked>,
    deaths:    MessageWriter<'w, OnDeathOccurred>,
    afflicted: MessageWriter<'w, FieldAfflicted>,
}

/// Drain occupants standing in fields and expire placements.
pub fn tick_fields(
    grid: Res<OccupancyGrid>,
    mut fields: ResMut<FieldRegistry>,
    mut occupants: FieldOccupants,
    mut signals: FieldDrainSignals,
    mut commands: Commands,
) {
    let placements: Vec<(CellLevel, FieldDef)> = fields
        .iter()
        .map(|(cell, placed)| (*cell, placed.def().clone()))
        .collect();

    let mut drained_this_round: bevy::platform::collections::HashSet<Entity> =
        bevy::platform::collections::HashSet::default();

    for (cell, def) in placements {
        let Some(occupant) = grid.occupant(&cell) else {
            continue;
        };
        let Ok((mut hp, mut life, wears)) = occupants.vitals.get_mut(occupant) else {
            continue;
        };
        if *life == LifeState::Dead {
            continue;
        }
        let consequences = FieldEffect::consequences_of(&def);
        let armor = OccupantArmor {
            wears,
            worn: &occupants.worn,
        };
        if consequences
            .iter()
            .any(|consequence| *consequence.exempts_occupant(&armor))
        {
            continue;
        }
        if !drained_this_round.contains(&occupant) {
            if occupants.ongoing.get(occupant).is_err() {
                signals.afflicted.write(FieldAfflicted::new(occupant, cell));
                commands.entity(occupant).insert(FieldOngoing);
            }
            drained_this_round.insert(occupant);
        }

        let mut drain = OccupantDrain {
            hp:     &mut hp,
            life:   &mut life,
            ticks:  &mut signals.ticks,
            deaths: &mut signals.deaths,
        };
        for consequence in &consequences {
            consequence.drain_occupant(cell, occupant, &mut drain);
        }
    }

    for entity in &occupants.ongoing {
        if !drained_this_round.contains(&entity) {
            commands.entity(entity).remove::<FieldOngoing>();
        }
    }

    fields.tick_down_and_expire();
}
