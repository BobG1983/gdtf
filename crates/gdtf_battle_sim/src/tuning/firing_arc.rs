use bevy::prelude::Deref;
use serde::Deserialize;

/// degrees, never a bare `f32`. Private inner + derived [`Deref`]; `#[serde(transparent)]`
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct FiringArc(f32);

impl FiringArc {
                                #[must_use]
    pub const fn new(degrees: f32) -> Self {
        Self(degrees)
    }
}

impl Default for FiringArc {
    fn default() -> Self {
        Self(120.0)
    }
}
