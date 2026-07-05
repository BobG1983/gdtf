//! The pan-feel tunable newtypes: speed, edge band, and the stick deadzone.

use bevy::prelude::*;
use serde::Deserialize;

/// The pan speed of the [`WorldCamera`](crate::WorldCamera), in world units per second.
///
/// A VIEW tunable (how fast the camera glides under player navigation), now MIGRATED into
/// the hot-reloadable [`PanTuning`](crate::PanTuning) table (GTW-299): the shipped value lives
/// as the
/// [`DEFAULT`](Self::DEFAULT) const here AND in `assets/core_tuning/pan.tuning.ron`, so editing
/// the `.ron` retunes the camera glide WITHOUT a rebuild. A newtype with a private inner
/// `f32` + derived [`Deref`](std::ops::Deref) (the house style for a domain value,
/// `no-bare-types.md`): the speed is a domain quantity (world-units/sec), never a bare `f32`.
/// `#[serde(transparent)]` + [`Deserialize`] so the `.ron` authors the inner number directly;
/// [`Default`] carries the shipped value so a missing `.ron` field degrades to the prior
/// behaviour rather than a parse error (mirrors [`FxTuning`](crate::FxTuning)'s newtypes).
#[derive(Deref, Clone, Copy, PartialEq, Debug, Deserialize)]
#[serde(transparent)]
pub struct PanSpeed(f32);

impl PanSpeed {
    /// The shipped default: `400` world-units/sec — the user's deliberate pan-feel tune (down
    /// from the old GTW-250 `600`), which was the ACTUAL existing runtime behaviour in the working
    /// tree before this migration. Preserved VERBATIM as the [`PanTuning`](crate::PanTuning)
    /// default so the
    /// migration leaves the camera glide exactly as the user had it (AC4: behaviour unchanged).
    pub const DEFAULT: f32 = 400.0;

    /// Construct a [`PanSpeed`] from world-units-per-second.
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

/// The mouse-edge band thickness, in logical screen pixels.
///
/// The cursor is "at an edge" (and pans the camera that way) when it sits within this many
/// logical pixels of a window edge. A VIEW tunable, now MIGRATED into the hot-reloadable
/// [`PanTuning`](crate::PanTuning) table (GTW-299): the shipped value lives as the
/// [`DEFAULT`](Self::DEFAULT)
/// const here AND in `assets/core_tuning/pan.tuning.ron`, so editing the `.ron` retunes the band
/// WITHOUT a rebuild. A newtype over a private `f32` ([`Deref`](std::ops::Deref)) per
/// `no-bare-types.md`. `#[serde(transparent)]` + [`Deserialize`] so the `.ron` authors the
/// inner number directly; [`Default`] carries the shipped value so a missing `.ron` field
/// degrades to the prior behaviour rather than a parse error.
#[derive(Deref, Clone, Copy, PartialEq, Debug, Deserialize)]
#[serde(transparent)]
pub struct EdgeBandPx(f32);

impl EdgeBandPx {
    /// The shipped default: `24` logical pixels — the migrated GTW-250 edge-band tune
    /// (preserved verbatim as the [`PanTuning`](crate::PanTuning) default so behaviour is
    /// unchanged).
    pub const DEFAULT: f32 = 24.0;

    /// Construct an [`EdgeBandPx`] from a logical-pixel band thickness.
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

/// The gamepad-stick deadzone: a stick magnitude at or below this contributes no pan.
///
/// A unitless `[0, 1]` analog-stick magnitude threshold below which the right stick is
/// treated as centred (no drift). A VIEW tunable (a presenter const, not `.ron`) and a
/// newtype over a private `f32` ([`Deref`](std::ops::Deref)) per `no-bare-types.md`.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct StickDeadzone(f32);

impl StickDeadzone {
    /// Construct a [`StickDeadzone`] from a unitless `[0, 1]` magnitude threshold.
    #[must_use]
    pub const fn new(magnitude: f32) -> Self {
        Self(magnitude)
    }
}

/// The shipping gamepad right-stick deadzone (see [`StickDeadzone`]): a stick magnitude at
/// or below this is ignored so a resting stick never drifts the camera.
pub const STICK_DEADZONE: StickDeadzone = StickDeadzone::new(0.15);
