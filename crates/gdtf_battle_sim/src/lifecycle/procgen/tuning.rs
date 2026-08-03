//! `#[serde(transparent)]` so it round-trips as a bare RON scalar. The values are
use bevy::{prelude::Resource, reflect::TypePath};
use serde::Deserialize;

use super::geometry::CellCount;

#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScatterCount(usize);

impl ScatterCount {
        #[must_use]
    pub const fn new(count: usize) -> Self {
        Self(count)
    }
}

/// `#[serde(transparent)]` parses a bare RON scalar. **Tunable** — tests assert only its
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct MinDensityFloor(f32);

impl MinDensityFloor {
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

/// derived [`Deref`](bevy::prelude::Deref); `#[serde(transparent)]` parses a bare RON scalar.
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct MaxCoverageCap(f32);

impl MaxCoverageCap {
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

/// `#[serde(transparent)]` parses a bare RON scalar. **Tunable** — tests assert only its
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct LargePrefabAreaThreshold(u32);

impl LargePrefabAreaThreshold {
            #[must_use]
    pub const fn new(area: u32) -> Self {
        Self(area)
    }

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

/// `#[serde(transparent)]` parses a bare RON scalar. **Tunable** — tests assert only its
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct DeadRectScatterCount(u8);

impl DeadRectScatterCount {
            #[must_use]
    pub const fn new(k: u8) -> Self {
        Self(k)
    }

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

/// (the same bound `CombatTuning` / the theme spec carry). `#[serde(default)]` so a file
#[derive(Debug, Clone, Copy, PartialEq, Default, Resource, Deserialize, TypePath)]
#[serde(default)]
pub struct ProcgenTuning {
            pub min_density_floor:           MinDensityFloor,
                pub max_coverage_cap:            MaxCoverageCap,
            pub large_prefab_area_threshold: LargePrefabAreaThreshold,
        pub dead_rect_scatter_count_k:   DeadRectScatterCount,
}
