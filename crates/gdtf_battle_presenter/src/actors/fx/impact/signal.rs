//! Shot impact resolved message for death despawn and impact FX.

use bevy::prelude::*;
use gdtf_battle_sim::resolve_and_apply::HitReport;

/// Fired when a projectile arrives and impact FX should play.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct ShotImpactResolved {
    /// Shooter entity that fired the round.
    pub shooter: Entity,
    /// Hit report if the round connected.
    pub report: Option<HitReport>,
}
