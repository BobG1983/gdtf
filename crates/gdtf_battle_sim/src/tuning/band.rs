use bevy::prelude::Deref;
use serde::Deserialize;

/// §"Banding"). `#[serde(transparent)]` lets it parse a bare RON scalar.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct BandEdge(f32);

impl BandEdge {
                #[must_use]
    pub const fn new(edge: f32) -> Self {
        Self(edge)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct ProjectileBandEdges {
        pub low_mid:  BandEdge,
        pub mid_high: BandEdge,
}

impl Default for ProjectileBandEdges {
    fn default() -> Self {
        Self {
            low_mid:  BandEdge(0.33),
            mid_high: BandEdge(0.67),
        }
    }
}
