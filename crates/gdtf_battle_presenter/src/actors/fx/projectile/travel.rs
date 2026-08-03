use bevy::prelude::*;
use gdtf_battle_sim::{
    prelude::{Cell, Level},
    resolve_and_apply::HitReport,
    weapon::DamageType,
};

use super::super::{fct::ClassifiedPop, tuning::ProjectileVelocity};

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShotProjectile;

#[derive(Component, Debug, Clone)]
pub struct ProjectileTravel {
        from:     Vec3,
        to:       Vec3,
                damage:   DamageType,
                    launch:   Timer,
                    velocity: ProjectileVelocity,
                traveled: f32,
                    pops:     Vec<ClassifiedPop>,
                    anchor:   (Cell, Level),
                                    shooter:  Entity,
                        report:   Option<HitReport>,
}

impl ProjectileTravel {
                                                                                                                                            #[expect(
        clippy::too_many_arguments,
        reason = "each is a distinct flight datum: the two endpoints, the damage type, the \
                  velocity, the stagger launch delay, the GTW-327 pops + anchor, and the GTW-328 \
                  shooter + report threaded to the impact signal — the ctor IS the bundle"
    )]
    #[must_use]
    pub(in crate::actors::fx) fn new(
        from: Vec3,
        to: Vec3,
        damage: DamageType,
        velocity: ProjectileVelocity,
        launch_delay: std::time::Duration,
        pops: Vec<ClassifiedPop>,
        anchor: (Cell, Level),
        shooter: Entity,
        report: Option<HitReport>,
    ) -> Self {
        Self {
            from,
            to,
            damage,
            launch: Timer::new(launch_delay, TimerMode::Once),
            velocity,
            traveled: 0.0,
            pops,
            anchor,
            shooter,
            report,
        }
    }

        #[must_use]
    fn distance(&self) -> f32 {
        self.from.distance(self.to)
    }

                                                pub fn advance(&mut self, delta: std::time::Duration) -> bool {
        if !self.launch.tick(delta).is_finished() {
            return false;
        }
        self.traveled = self.velocity.mul_add(delta.as_secs_f32(), self.traveled);
        self.traveled >= self.distance()
    }

                #[must_use]
    pub fn launched(&self) -> bool {
        self.launch.is_finished()
    }

                    #[must_use]
    pub fn fraction(&self) -> f32 {
        let distance = self.distance();
        if distance <= f32::EPSILON {
            return 1.0;
        }
        (self.traveled / distance).clamp(0.0, 1.0)
    }

            #[must_use]
    pub fn position(&self) -> Vec3 {
        self.from.lerp(self.to, self.fraction())
    }

        #[must_use]
    pub const fn arrival(&self) -> Vec3 {
        self.to
    }

            #[must_use]
    pub const fn damage(&self) -> DamageType {
        self.damage
    }

                    #[must_use]
    pub(super) fn take_pops(&mut self) -> Vec<ClassifiedPop> {
        std::mem::take(&mut self.pops)
    }

            #[must_use]
    pub(super) const fn anchor(&self) -> (Cell, Level) {
        self.anchor
    }

                #[must_use]
    pub(super) const fn shooter(&self) -> Entity {
        self.shooter
    }

                    #[must_use]
    pub(super) fn report(&self) -> Option<HitReport> {
        self.report.clone()
    }
}
