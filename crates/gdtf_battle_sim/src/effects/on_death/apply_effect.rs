use bevy::prelude::Query;

use crate::{
    effects::{
        fields::{FieldDefRegistry, FieldRegistry},
        on_death::OnDeathOccurred,
    },
    ganger::{Hp, LifeState},
    metric::CellLevel,
    occupancy::OccupancyGrid,
};

pub type VictimRow = (&'static mut Hp, &'static mut LifeState);

pub struct DeathFanOut<'a, 'w, 's> {
        pub grid:       &'a OccupancyGrid,
        pub victims:    &'a mut Query<'w, 's, VictimRow>,
        pub fields:     &'a mut FieldRegistry,
                pub field_defs: Option<&'a FieldDefRegistry>,
            pub cascade:    &'a mut Vec<OnDeathOccurred>,
}

pub trait ApplyOnDeathEffect {
                                        fn fan_at(&self, at: CellLevel, fan_out: &mut DeathFanOut<'_, '_, '_>);
}
