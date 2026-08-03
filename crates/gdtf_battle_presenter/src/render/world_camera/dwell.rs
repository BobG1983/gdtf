//! Edge-pan dwell timers for mouse and gamepad cursor.

use bevy::prelude::*;

/// Accumulated seconds the cursor has stayed on a pan edge.
#[derive(Debug, Clone, Copy, PartialEq, Deref)]
pub struct DwellElapsed(f32);

impl DwellElapsed {
    /// Zero elapsed dwell.
    pub const ZERO: Self = Self(0.0);

    /// Add `delta` seconds of dwell.
    pub fn accumulate(&mut self, delta: f32) {
        self.0 += delta;
    }

    /// Clear accumulated dwell.
    pub const fn reset(&mut self) {
        self.0 = 0.0;
    }
}

impl Default for DwellElapsed {
    fn default() -> Self {
        Self::ZERO
    }
}

/// Separate mouse and gamepad edge-dwell accumulators.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub struct PanEdgeDwellState {
    /// Mouse cursor edge dwell.
    pub mouse: DwellElapsed,
    /// Virtual gamepad cursor edge dwell.
    pub gamepad: DwellElapsed,
}

/// Whether accumulated dwell has met the configured threshold.
#[must_use]
pub fn should_edge_pan_after_dwell(
    accumulated: DwellElapsed,
    threshold: super::tuning::DwellDelaySeconds,
) -> bool {
    *accumulated >= *threshold
}
