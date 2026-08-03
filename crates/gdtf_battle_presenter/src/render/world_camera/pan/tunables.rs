use bevy::prelude::*;
use serde::Deserialize;

/// `#[serde(transparent)]` + [`Deserialize`] so the `.ron` authors the inner number directly;
#[derive(Deref, Clone, Copy, PartialEq, Debug, Deserialize)]
#[serde(transparent)]
pub struct PanSpeed(f32);

impl PanSpeed {
                        pub const DEFAULT: f32 = 400.0;

        #[must_use]
    pub const fn new(units_per_second: f32) -> Self {
        Self(units_per_second)
    }
}

impl Default for PanSpeed {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

/// `no-bare-types.md`. `#[serde(transparent)]` + [`Deserialize`] so the `.ron` authors the
#[derive(Deref, Clone, Copy, PartialEq, Debug, Deserialize)]
#[serde(transparent)]
pub struct EdgeBandPx(f32);

impl EdgeBandPx {
                pub const DEFAULT: f32 = 24.0;

        #[must_use]
    pub const fn new(pixels: f32) -> Self {
        Self(pixels)
    }
}

impl Default for EdgeBandPx {
    fn default() -> Self {
        Self(Self::DEFAULT)
    }
}

#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct StickDeadzone(f32);

impl StickDeadzone {
        #[must_use]
    pub const fn new(magnitude: f32) -> Self {
        Self(magnitude)
    }
}

pub const STICK_DEADZONE: StickDeadzone = StickDeadzone::new(0.15);
