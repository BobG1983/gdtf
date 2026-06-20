//! The projectile clearance band edges (battle-space.md §"Banding").

use bevy::prelude::Deref;
use serde::Deserialize;

/// A projectile clearance band edge, as a **level-fraction** — a dimensionless
/// fraction of one level's height (`z ∈ [0,1)` within a storey).
///
/// One newtype shared by **both** band edges of [`ProjectileBandEdges`]: the two
/// edges are the same *kind* of value (a level-fraction clearance threshold),
/// distinguished by their field. Because they are fractions of a storey they
/// re-scale with the cubic voxel and carry no pixel (`docs/combat/battle-space.md`
/// §"Banding"). `#[serde(transparent)]` lets it parse a bare RON scalar.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct BandEdge(f32);

impl BandEdge {
    /// Build a band-edge level-fraction from its magnitude (a fraction of one
    /// level's height) — for tests and programmatic tuning edits; shipped values
    /// come from the `.ron` via the derived [`Deserialize`].
    #[must_use]
    pub const fn new(edge: f32) -> Self {
        Self(edge)
    }
}

/// The projectile clearance band edges, as **level-fractions** within one
/// level's height.
///
/// The march bands each crossed cell LOW / MID / HIGH by the round's continuous
/// `z` within the crossed level and compares it to the occupant's band
/// (resolution.md §2). The edges are dimensionless fractions of a storey
/// (defaults ≈ ⅓ and ⅔ of a level), so they re-scale with the cubic voxel and
/// carry no pixel (`docs/combat/battle-space.md` §"Banding"). `low_mid` is the
/// LOW→MID edge, `mid_high` the MID→HIGH edge.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct ProjectileBandEdges {
    /// The LOW→MID clearance edge (level-fraction). Default ≈ ⅓ of a level.
    pub low_mid:  BandEdge,
    /// The MID→HIGH clearance edge (level-fraction). Default ≈ ⅔ of a level.
    pub mid_high: BandEdge,
}

impl Default for ProjectileBandEdges {
    fn default() -> Self {
        // Tunable level-fraction defaults (≈ ⅓ and ⅔ of a level) from
        // docs/combat/battle-space.md §"Banding". These are balance data, not a
        // coordinate-system fact — value-agnostic tests only.
        Self {
            low_mid:  BandEdge(0.33),
            mid_high: BandEdge(0.67),
        }
    }
}
