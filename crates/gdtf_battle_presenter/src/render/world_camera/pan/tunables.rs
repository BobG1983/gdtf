//! Newtypes for pan speed, edge band, and stick deadzone.

use bevy::prelude::*;
use serde::Deserialize;

/// World units per second at full pan input.
///
/// `#[serde(transparent)]` so the `.ron` authors the inner number directly.
#[derive(Deref, Clone, Copy, PartialEq, Debug, Deserialize)]
#[serde(transparent)]
pub struct PanSpeed(f32);

impl PanSpeed {
    /// Shipped default pan speed.
    pub const DEFAULT: f32 = 400.0;

    /// Build from units per second.
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

/// Pixel width of the screen-edge pan band.
///
/// `#[serde(transparent)]` so the `.ron` authors the inner number directly.
#[derive(Deref, Clone, Copy, PartialEq, Debug, Deserialize)]
#[serde(transparent)]
pub struct EdgeBandPx(f32);

impl EdgeBandPx {
    /// Shipped default edge band in pixels.
    pub const DEFAULT: f32 = 24.0;

    /// Build from a pixel width.
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

/// Stick magnitude below which stick input is ignored.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct StickDeadzone(f32);

impl StickDeadzone {
    /// Build from a unit-circle magnitude threshold.
    #[must_use]
    pub const fn new(magnitude: f32) -> Self {
        Self(magnitude)
    }
}

/// Shipped stick deadzone.
pub const STICK_DEADZONE: StickDeadzone = StickDeadzone::new(0.15);
