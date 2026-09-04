use bevy::prelude::*;

use crate::states::running::game::battlescape::combat_log::tuning::{
    FadeInSeconds, FadeOutSeconds, LineTtlSeconds,
};

#[derive(Component, Debug, Clone)]
pub(crate) struct LogLineFade {
    ttl:        Timer,
    life:       f32,
    fade_in:    f32,
    fade_out:   f32,
    base_alpha: f32,
}

impl LogLineFade {
    #[must_use]
    pub(crate) fn new(
        ttl: LineTtlSeconds,
        fade_in: FadeInSeconds,
        fade_out: FadeOutSeconds,
        base_alpha: f32,
    ) -> Self {
        let life = (*ttl).max(0.0);
        let mut fade_in = (*fade_in).max(0.0);
        let mut fade_out = (*fade_out).max(0.0);
        let total = fade_in + fade_out;
        if total > life && total > f32::EPSILON {
            let scale = life / total;
            fade_in *= scale;
            fade_out *= scale;
        }
        Self {
            ttl: Timer::from_seconds(life, TimerMode::Once),
            life,
            fade_in,
            fade_out,
            base_alpha,
        }
    }

    pub(crate) fn advance(&mut self, delta: std::time::Duration) -> bool {
        self.ttl.tick(delta).is_finished()
    }

    #[must_use]
    pub(crate) fn alpha(&self) -> f32 {
        let elapsed = self.ttl.elapsed_secs();
        if elapsed < self.fade_in && self.fade_in > f32::EPSILON {
            return (self.base_alpha * (elapsed / self.fade_in)).clamp(0.0, self.base_alpha);
        }
        let fade_out_start = self.life - self.fade_out;
        if elapsed >= fade_out_start && self.fade_out > f32::EPSILON {
            let into_fade = (elapsed - fade_out_start) / self.fade_out;
            return (self.base_alpha * (1.0 - into_fade)).clamp(0.0, self.base_alpha);
        }
        self.base_alpha
    }
}
