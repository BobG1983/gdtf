//! A buffered Bevy **message** (`#[derive(Message)]`), mirroring
use bevy::prelude::{Entity, Message};

use crate::{
    metric::{Cell, Level, SimPos},
    resolve_and_apply::HitReport,
    resolve_coarse::{ShotKind, ShotOutcome},
    sample_cone::ShotDir,
    weapon::DamageType,
};

#[derive(Message, Debug, Clone, PartialEq)]
pub struct ShotFired {
        pub shooter:      Entity,
            pub muzzle:       SimPos,
            pub trajectory:   ShotDir,
            pub impact_cell:  Cell,
        pub impact_level: Level,
            pub kind:         ShotKind,
                            pub damage:       DamageType,
                                                                            pub report:       Option<HitReport>,
}

impl ShotFired {
                                                                    #[must_use]
    pub const fn from_outcome(shooter: Entity, damage: DamageType, outcome: &ShotOutcome) -> Self {
        Self {
            shooter,
            muzzle: outcome.muzzle,
            trajectory: outcome.trajectory,
            impact_cell: outcome.cell,
            impact_level: outcome.level,
            kind: outcome.kind,
            damage,
            report: None,
        }
    }

                                                #[must_use]
    pub const fn from_round(
        shooter: Entity,
        damage: DamageType,
        outcome: &ShotOutcome,
        report: HitReport,
    ) -> Self {
        Self {
            shooter,
            muzzle: outcome.muzzle,
            trajectory: outcome.trajectory,
            impact_cell: outcome.cell,
            impact_level: outcome.level,
            kind: outcome.kind,
            damage,
            report: Some(report),
        }
    }
}
