//! In-flight shot projectile motion.

use std::time::Duration;

use bevy::prelude::*;
use gdtf_battle_sim::{
    prelude::{Cell, Level},
    resolve_and_apply::HitReport,
    weapon::DamageType,
};

use super::super::{fct::ClassifiedPop, tuning::ProjectileVelocity};

/// Marker on a flying shot projectile entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShotProjectile;

/// Where a projectile flies from, to, how fast, and after what stagger.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::actors::fx) struct ProjectileFlight {
    from:         Vec3,
    to:           Vec3,
    velocity:     ProjectileVelocity,
    launch_delay: Duration,
}

impl ProjectileFlight {
    /// Build a muzzle-to-target path.
    #[must_use]
    pub(in crate::actors::fx) const fn new(
        from: Vec3,
        to: Vec3,
        velocity: ProjectileVelocity,
        launch_delay: Duration,
    ) -> Self {
        Self {
            from,
            to,
            velocity,
            launch_delay,
        }
    }
}

/// What a projectile delivers where it lands.
#[derive(Debug, Clone)]
pub(in crate::actors::fx) struct ImpactPayload {
    damage:  DamageType,
    pops:    Vec<ClassifiedPop>,
    anchor:  (Cell, Level),
    shooter: Entity,
    report:  Option<HitReport>,
}

impl ImpactPayload {
    /// Build the impact FX, pops, and report a shot carries.
    #[must_use]
    pub(in crate::actors::fx) const fn new(
        damage: DamageType,
        pops: Vec<ClassifiedPop>,
        anchor: (Cell, Level),
        shooter: Entity,
        report: Option<HitReport>,
    ) -> Self {
        Self {
            damage,
            pops,
            anchor,
            shooter,
            report,
        }
    }
}

/// Flight path, stagger, and payload for a shot projectile.
#[derive(Component, Debug, Clone)]
pub struct ProjectileTravel {
    flight:   ProjectileFlight,
    launch:   Timer,
    traveled: f32,
    payload:  ImpactPayload,
}

impl ProjectileTravel {
    #[must_use]
    pub(in crate::actors::fx) fn new(flight: ProjectileFlight, payload: ImpactPayload) -> Self {
        Self {
            launch: Timer::new(flight.launch_delay, TimerMode::Once),
            flight,
            traveled: 0.0,
            payload,
        }
    }

    #[must_use]
    fn distance(&self) -> f32 {
        self.flight.from.distance(self.flight.to)
    }

    /// Advance flight. Returns `true` when the projectile has arrived.
    pub fn advance(&mut self, delta: Duration) -> bool {
        if !self.launch.tick(delta).is_finished() {
            return false;
        }
        self.traveled = self
            .flight
            .velocity
            .mul_add(delta.as_secs_f32(), self.traveled);
        self.traveled >= self.distance()
    }

    /// Whether the launch stagger has finished.
    #[must_use]
    pub fn launched(&self) -> bool {
        self.launch.is_finished()
    }

    /// Fraction of the path traveled, clamped to `[0, 1]`.
    #[must_use]
    pub fn fraction(&self) -> f32 {
        let distance = self.distance();
        if distance <= f32::EPSILON {
            return 1.0;
        }
        (self.traveled / distance).clamp(0.0, 1.0)
    }

    /// Current interpolated world position.
    #[must_use]
    pub fn position(&self) -> Vec3 {
        self.flight.from.lerp(self.flight.to, self.fraction())
    }

    /// Arrival world position.
    #[must_use]
    pub const fn arrival(&self) -> Vec3 {
        self.flight.to
    }

    /// Damage type for impact FX.
    #[must_use]
    pub const fn damage(&self) -> DamageType {
        self.payload.damage
    }

    #[must_use]
    pub(super) fn take_pops(&mut self) -> Vec<ClassifiedPop> {
        std::mem::take(&mut self.payload.pops)
    }

    #[must_use]
    pub(super) const fn anchor(&self) -> (Cell, Level) {
        self.payload.anchor
    }

    #[must_use]
    pub(super) const fn shooter(&self) -> Entity {
        self.payload.shooter
    }

    #[must_use]
    pub(super) fn report(&self) -> Option<HitReport> {
        self.payload.report.clone()
    }
}
