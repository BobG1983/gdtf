use bevy::prelude::{Deref, Entity, MessageWriter, Mut, Query, With};

use super::FieldTurns;
use crate::{
    armor::{ArmorType, Wears, WornBy},
    effects::{fields::FieldTicked, on_death::OnDeathOccurred},
    ganger::{Hp, LifeState},
    metric::CellLevel,
};

pub struct OccupantArmor<'a, 'w, 's> {
        pub wears: &'a Wears,
        pub worn:  &'a Query<'w, 's, &'static ArmorType, With<WornBy>>,
}

pub struct OccupantDrain<'a, 'hp, 'life, 'wt, 'wd> {
        pub hp:     &'a mut Mut<'hp, Hp>,
        pub life:   &'a mut Mut<'life, LifeState>,
        pub ticks:  &'a mut MessageWriter<'wt, FieldTicked>,
                pub deaths: &'a mut MessageWriter<'wd, OnDeathOccurred>,
}

pub trait ApplyFieldEffect {
                        fn exempts_occupant(&self, _armor: &OccupantArmor<'_, '_, '_>) -> DrainExempt {
        DrainExempt(false)
    }

                                        fn drain_occupant(
        &self,
        _at: CellLevel,
        _occupant: Entity,
        _drain: &mut OccupantDrain<'_, '_, '_, '_, '_>,
    ) {
    }

                                fn initial_countdown(&self) -> Option<FieldTurns> {
        None
    }

                                        fn count_down_one_turn(&self, _remaining: &mut Option<FieldTurns>) -> FieldExpired {
        FieldExpired(false)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrainExempt(bool);

impl DrainExempt {
                #[must_use]
    pub const fn new(exempt: bool) -> Self {
        Self(exempt)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldExpired(bool);

impl FieldExpired {
                #[must_use]
    pub const fn new(expired: bool) -> Self {
        Self(expired)
    }
}
