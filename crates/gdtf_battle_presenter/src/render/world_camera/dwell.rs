use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Deref)]
pub struct DwellElapsed(f32);

impl DwellElapsed {
        pub const ZERO: Self = Self(0.0);

                    pub fn accumulate(&mut self, delta: f32) {
        self.0 += delta;
    }

            pub const fn reset(&mut self) {
        self.0 = 0.0;
    }
}

impl Default for DwellElapsed {
    fn default() -> Self {
        Self::ZERO
    }
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub struct PanEdgeDwellState {
        pub mouse:   DwellElapsed,
            pub gamepad: DwellElapsed,
}

#[must_use]
pub fn should_edge_pan_after_dwell(
    accumulated: DwellElapsed,
    threshold: super::tuning::DwellDelaySeconds,
) -> bool {
    *accumulated >= *threshold
}
