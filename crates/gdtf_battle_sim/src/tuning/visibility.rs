//! These are **sim-authored** balance leaves: the squad fog is computed model-side
use bevy::prelude::Deref;
use serde::Deserialize;

/// `#[serde(transparent)]` lets it parse a bare RON scalar (the tuning-leaf precedent).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct ViewRange(u16);

impl ViewRange {
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

/// **DEPRECATED / UNUSED by the renderer as of GTW-348.** The user changed the EXPLORED
/// a bare `f32`. Private inner + derived [`Deref`]; `#[serde(transparent)]`. **Tunable**
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct ExploredDim(f32);

impl ExploredDim {
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
