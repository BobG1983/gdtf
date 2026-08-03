use bevy::prelude::*;
use gdtf_battle_sim::resolve_and_apply::HitReport;

#[derive(Message, Debug, Clone, PartialEq)]
pub struct ShotImpactResolved {
        pub shooter: Entity,
                        pub report:  Option<HitReport>,
}
