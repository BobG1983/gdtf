//! Trait and helpers for applying field effects to occupants.

use bevy::prelude::{Deref, Entity, MessageWriter, Mut, Query, With};

use super::FieldTurns;
use crate::{
    armor::{ArmorType, Wears, WornBy},
    effects::{fields::FieldTicked, on_death::OnDeathOccurred},
    ganger::{Hp, LifeState},
    metric::CellLevel,
};

/// Armor worn by an occupant, for immunity checks.
pub struct OccupantArmor<'a, 'w, 's> {
    /// Wears relationship on the ganger.
    pub wears: &'a Wears,
    /// Armor type components on worn pieces.
    pub worn:  &'a Query<'w, 's, &'static ArmorType, With<WornBy>>,
}

/// Mutable handles used when draining an occupant.
pub struct OccupantDrain<'a, 'hp, 'life, 'wt, 'wd> {
    /// Hit points.
    pub hp:     &'a mut Mut<'hp, Hp>,
    /// Life state.
    pub life:   &'a mut Mut<'life, LifeState>,
    /// Field tick messages.
    pub ticks:  &'a mut MessageWriter<'wt, FieldTicked>,
    /// Death messages.
    pub deaths: &'a mut MessageWriter<'wd, OnDeathOccurred>,
}

/// Behaviour a field consequence can apply.
pub trait ApplyFieldEffect {
    /// Whether this occupant is exempt from drain.
    fn exempts_occupant(&self, _armor: &OccupantArmor<'_, '_, '_>) -> DrainExempt {
        DrainExempt(false)
    }

    /// Drain the occupant if not exempt.
    fn drain_occupant(
        &self,
        _at: CellLevel,
        _occupant: Entity,
        _drain: &mut OccupantDrain<'_, '_, '_, '_, '_>,
    ) {
    }

    /// Initial countdown when the field is placed.
    fn initial_countdown(&self) -> Option<FieldTurns> {
        None
    }

    /// Count down one turn; return whether the field expired.
    fn count_down_one_turn(&self, _remaining: &mut Option<FieldTurns>) -> FieldExpired {
        FieldExpired(false)
    }
}

/// Whether an occupant is exempt from field drain.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrainExempt(bool);

impl DrainExempt {
    /// Wrap the flag.
    #[must_use]
    pub const fn new(exempt: bool) -> Self {
        Self(exempt)
    }
}

/// Whether a field placement has expired.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldExpired(bool);

impl FieldExpired {
    /// Wrap the flag.
    #[must_use]
    pub const fn new(expired: bool) -> Self {
        Self(expired)
    }
}
