use bevy::prelude::*;
use gdtf_battle_sim::{
    prelude::{Cell, Level},
    resolve_and_apply::HitReport,
    weapon::DamageType,
};

use super::super::fct::ClassifiedPop;

#[derive(Component, Debug, Clone)]
pub struct PendingImpact {
        pub(in crate::actors::fx) at:      Vec3,
        pub(in crate::actors::fx) damage:  DamageType,
                    pub(in crate::actors::fx) pops:    Vec<ClassifiedPop>,
            pub(in crate::actors::fx) anchor:  (Cell, Level),
                            pub(in crate::actors::fx) shooter: Entity,
                    pub(in crate::actors::fx) report:  Option<HitReport>,
}

impl PendingImpact {
                                                                                #[must_use]
    pub(in crate::actors::fx) const fn for_blast(at: Vec3, damage: DamageType) -> Self {
        Self {
            at,
            damage,
            pops: Vec::new(),
            anchor: (Cell::new(0, 0), Level::new(0)),
            shooter: Entity::PLACEHOLDER,
            report: None,
        }
    }
}
