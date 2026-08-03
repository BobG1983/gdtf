//! Path and vision blocking derived from terrain defs.

use bevy::prelude::Deref;

use super::{LosBlocking, TerrainDef, TerrainSimKind, TerrainTag};
use crate::{
    cover::HeightBand,
    occupancy::{OccludesVision, PathBlocked},
    terrain::entity::TerrainPieceKind,
};

/// Whether a piece can be opened (door/hatch).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Openable(bool);

impl Openable {
    /// Wrap the flag.
    #[must_use]
    pub const fn new(openable: bool) -> Self {
        Self(openable)
    }
}

/// Path blocking for a def (override or tag/kind default).
#[must_use]
pub fn derives_path_blocking(def: &TerrainDef) -> PathBlocked {
    if let Some(over) = def.blocks_pathing {
        return PathBlocked::new(*over);
    }
    let explicit = def.tags.contains(&TerrainTag::BlocksPathfinding);
    PathBlocked::new(explicit || *sim_kind_blocks_path(&def.sim_kind))
}

/// Default path blocking from sim kind alone.
#[must_use]
pub const fn sim_kind_blocks_path(sim_kind: &TerrainSimKind) -> PathBlocked {
    PathBlocked::new(matches!(
        sim_kind.kind(),
        TerrainPieceKind::Wall | TerrainPieceKind::Cover | TerrainPieceKind::Emplacement
    ))
}

/// Vision occlusion band for a def, if any.
#[must_use]
pub fn derives_vision_occlusion(def: &TerrainDef) -> Option<HeightBand> {
    los_blocking_to_band(resolved_los_blocking(def), &def.sim_kind)
}

/// Resolved LOS blocking (override, tag, or kind default).
#[must_use]
pub fn resolved_los_blocking(def: &TerrainDef) -> LosBlocking {
    if let Some(over) = def.blocks_los {
        return over;
    }
    if def.tags.contains(&TerrainTag::BlocksVision) {
        return match sim_kind_band(&def.sim_kind) {
            Some(_) => LosBlocking::UpToHeightBand,
            None => LosBlocking::Full,
        };
    }
    sim_kind_default_los(&def.sim_kind)
}

/// Default LOS blocking from sim kind alone.
#[must_use]
pub const fn sim_kind_default_los(sim_kind: &TerrainSimKind) -> LosBlocking {
    match sim_kind.kind() {
        TerrainPieceKind::Wall => LosBlocking::Full,
        TerrainPieceKind::Cover | TerrainPieceKind::Emplacement => LosBlocking::UpToHeightBand,
        TerrainPieceKind::Slab => LosBlocking::None,
    }
}

/// Convert LOS blocking to an optional height band.
#[must_use]
pub const fn los_blocking_to_band(
    los: LosBlocking,
    sim_kind: &TerrainSimKind,
) -> Option<HeightBand> {
    match los {
        LosBlocking::Full => Some(HeightBand::High),
        LosBlocking::UpToHeightBand => match sim_kind_band(sim_kind) {
            Some(band) => Some(band),
            None => Some(HeightBand::High),
        },
        LosBlocking::None => None,
    }
}

/// Whether the sim kind occludes vision by default.
#[must_use]
pub const fn sim_kind_occludes_vision(sim_kind: &TerrainSimKind) -> OccludesVision {
    OccludesVision::new(matches!(
        sim_kind.kind(),
        TerrainPieceKind::Wall | TerrainPieceKind::Cover | TerrainPieceKind::Emplacement
    ))
}

const fn sim_kind_band(sim_kind: &TerrainSimKind) -> Option<HeightBand> {
    match sim_kind {
        TerrainSimKind::Wall { height_band, .. }
        | TerrainSimKind::Cover { height_band, .. }
        | TerrainSimKind::Emplacement { height_band, .. } => Some(*height_band),
        TerrainSimKind::Slab { .. } => None,
    }
}

/// Whether the def is tagged openable.
#[must_use]
pub fn is_openable(def: &TerrainDef) -> Openable {
    Openable::new(def.tags.contains(&TerrainTag::Openable))
}

/// Vision band when a closed openable blocks sight.
#[must_use]
pub fn closed_openable_vision_band(def: &TerrainDef) -> HeightBand {
    sim_kind_band(&def.sim_kind).unwrap_or(HeightBand::High)
}
