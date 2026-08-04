//! Soft findings and the final emitted level.

use super::deploy::DeploymentZones;
use crate::{level::ThemeUuid, situation::Situation, terrain::def::TerrainUuid};

/// Non-fatal issue found while generating a level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcgenFinding {
    /// Theme has no default floor.
    MissingThemeDefaultFloor {
        /// Theme key.
        theme: ThemeUuid,
    },
    /// Terrain piece could not be resolved.
    UnresolvedTerrainPiece {
        /// Piece key.
        piece: TerrainUuid,
    },
}

/// Generated situation plus findings and deployment zones.
#[derive(Debug, Clone)]
pub struct EmittedLevel {
    /// Authored situation ready for setup.
    pub situation: Situation,
    /// Soft findings from generation.
    pub findings:  Vec<ProcgenFinding>,
    /// Player and enemy deployment zones.
    pub zones:     DeploymentZones,
}
