//! View range and explored-dim factor.

use bevy::prelude::Deref;
use serde::Deserialize;

/// Maximum sight range in cells.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct ViewRange(u16);

impl ViewRange {
    /// Wrap a cell count.
    #[must_use]
    pub const fn new(cells: u16) -> Self {
        Self(cells)
    }
}

impl Default for ViewRange {
    fn default() -> Self {
        Self(14)
    }
}

/// Dim factor for explored-but-not-visible tiles (legacy / optional).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ExploredDim(f32);

impl ExploredDim {
    /// Wrap a dim factor.
    #[must_use]
    pub const fn new(factor: f32) -> Self {
        Self(factor)
    }
}

impl Default for ExploredDim {
    fn default() -> Self {
        Self(0.55)
    }
}
