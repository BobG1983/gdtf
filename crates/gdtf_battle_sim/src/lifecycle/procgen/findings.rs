use super::deploy::DeploymentZones;
use crate::{level::ThemeUuid, situation::Situation, terrain::def::TerrainUuid};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcgenFinding {
                        MissingThemeDefaultFloor {
                theme: ThemeUuid,
    },
                        UnresolvedTerrainPiece {
                piece: TerrainUuid,
    },
}

#[derive(Debug, Clone)]
pub struct EmittedLevel {
        pub situation: Situation,
            pub findings:  Vec<ProcgenFinding>,
                        pub zones:     DeploymentZones,
}
