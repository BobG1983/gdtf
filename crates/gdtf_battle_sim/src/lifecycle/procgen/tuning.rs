//! Procgen density and scatter knobs.

use bevy::{prelude::Resource, reflect::TypePath};
use serde::Deserialize;

use super::geometry::CellCount;

/// How many scatter attempts to make.
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScatterCount(usize);

impl ScatterCount {
    /// Wrap a count.
    #[must_use]
    pub const fn new(count: usize) -> Self {
        Self(count)
    }
}

/// Minimum occupied fraction of the board.
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct MinDensityFloor(f32);

impl MinDensityFloor {
    /// Wrap a fraction.
    #[must_use]
    pub const fn new(fraction: f32) -> Self {
        Self(fraction)
    }
}

impl Default for MinDensityFloor {
    fn default() -> Self {
        Self(0.45)
    }
}

/// Maximum occupied fraction of the board.
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct MaxCoverageCap(f32);

impl MaxCoverageCap {
    /// Wrap a fraction.
    #[must_use]
    pub const fn new(fraction: f32) -> Self {
        Self(fraction)
    }
}

impl Default for MaxCoverageCap {
    fn default() -> Self {
        Self(0.85)
    }
}

/// Prefabs larger than this area count as "large".
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct LargePrefabAreaThreshold(u32);

impl LargePrefabAreaThreshold {
    /// Wrap an area.
    #[must_use]
    pub const fn new(area: u32) -> Self {
        Self(area)
    }

    /// As a cell count.
    #[must_use]
    pub const fn area(self) -> CellCount {
        CellCount::new(self.0 as i64)
    }
}

impl Default for LargePrefabAreaThreshold {
    fn default() -> Self {
        Self(64)
    }
}

/// Scatter attempts into leftover dead space.
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct DeadRectScatterCount(u8);

impl DeadRectScatterCount {
    /// Wrap a count.
    #[must_use]
    pub const fn new(k: u8) -> Self {
        Self(k)
    }

    /// As a scatter count.
    #[must_use]
    pub const fn count(self) -> ScatterCount {
        ScatterCount::new(self.0 as usize)
    }
}

impl Default for DeadRectScatterCount {
    fn default() -> Self {
        Self(3)
    }
}

/// All procgen density and scatter knobs.
#[derive(Debug, Clone, Copy, PartialEq, Default, Resource, Deserialize, TypePath)]
#[serde(default)]
pub struct ProcgenTuning {
    /// Minimum board occupancy fraction.
    pub min_density_floor:           MinDensityFloor,
    /// Maximum board occupancy fraction.
    pub max_coverage_cap:            MaxCoverageCap,
    /// Area above which a prefab is "large".
    pub large_prefab_area_threshold: LargePrefabAreaThreshold,
    /// Scatter attempts into dead rects.
    pub dead_rect_scatter_count_k:   DeadRectScatterCount,
}
