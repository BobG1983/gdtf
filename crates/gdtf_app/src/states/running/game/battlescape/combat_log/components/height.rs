use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct PanelHeightAnim {
    current: f32,
}

impl PanelHeightAnim {
    #[must_use]
    pub(crate) const fn new(current: f32) -> Self {
        Self { current }
    }

    #[must_use]
    pub(crate) const fn current(self) -> f32 {
        self.current
    }

    pub(crate) fn ease_toward(&mut self, target: f32, factor: f32) {
        self.current = (target - self.current).mul_add(factor.clamp(0.0, 1.0), self.current);
        if (target - self.current).abs() < f32::EPSILON {
            self.current = target;
        }
    }
}
