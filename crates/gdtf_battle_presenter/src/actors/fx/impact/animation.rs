use bevy::prelude::*;
use gdtf_battle_sim::weapon::DamageType;

use super::super::{roles::IMPACT_FRAME_COUNT, tuning::ImpactFrameSeconds};

pub(super) const IMPACT_FRAME_SCALES: [f32; IMPACT_FRAME_COUNT] = [1.1, 1.5, 1.9];

pub(super) fn impact_frame_scale(frame: usize) -> f32 {
    IMPACT_FRAME_SCALES
        .get(frame)
        .copied()
        .or_else(|| IMPACT_FRAME_SCALES.last().copied())
        .unwrap_or(1.0)
}

#[derive(Component, Debug, Clone)]
pub struct ImpactAnimation {
        damage:        DamageType,
        frame:         usize,
                frame_seconds: ImpactFrameSeconds,
        clock:         Timer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ImpactStep {
        Showing(usize),
        Finished,
}

impl ImpactAnimation {
                                            #[must_use]
    pub fn new(damage: DamageType, frame_seconds: ImpactFrameSeconds) -> Self {
        Self {
            damage,
            frame: 0,
            frame_seconds,
            clock: Timer::from_seconds(*frame_seconds, TimerMode::Once),
        }
    }

        #[must_use]
    pub const fn damage(&self) -> DamageType {
        self.damage
    }

                                                pub(super) fn advance(&mut self, delta: std::time::Duration) -> ImpactStep {
        if !self.clock.tick(delta).is_finished() {
            return ImpactStep::Showing(self.frame);
        }
        self.frame += 1;
        if self.frame >= IMPACT_FRAME_COUNT {
            return ImpactStep::Finished;
        }
        self.clock = Timer::from_seconds(*self.frame_seconds, TimerMode::Once);
        ImpactStep::Showing(self.frame)
    }
}
