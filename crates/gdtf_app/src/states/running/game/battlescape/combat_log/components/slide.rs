use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct LineSlide {
    current: f32,
}

impl LineSlide {
    #[must_use]
    pub(crate) const fn new(offset: f32) -> Self {
        Self { current: offset }
    }

    #[must_use]
    pub(crate) const fn current(self) -> f32 {
        self.current
    }

    pub(crate) fn displace(&mut self, delta: f32) {
        self.current += delta;
    }

    pub(crate) fn ease_toward_target(&mut self, factor: f32) {
        self.current *= 1.0 - factor.clamp(0.0, 1.0);
        if self.current.abs() < f32::EPSILON {
            self.current = 0.0;
        }
    }
}
