//! What the connector pairing pass did after a tile landed, on the wire.

use serde::{Deserialize, Serialize};

use super::{cell::EditorCellLevelNet, key::TerrainKeyNet};
use crate::connector_pairing::PairingOutcome;

/// Whether a paint placed one tile, two, or none.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum PairingOutcomeNet {
    /// The rules turned the placement down, so nothing was written.
    Rejected,
    /// The tile landed and pairs with nothing.
    PlacedNoPair,
    /// The tile landed, and its paired down connector could not follow.
    PlacedPairSkipped,
    /// The tile landed, and a second tile went in one storey above it.
    PairPlaced {
        /// The down connector the pass chose.
        down: TerrainKeyNet,
        /// The slot that second tile landed in.
        at:   EditorCellLevelNet,
    },
}

impl PairingOutcomeNet {
    /// Mirror the editor's own outcome, with no wildcard arm.
    pub(in crate::net_qa) fn from_outcome(outcome: PairingOutcome) -> Self {
        match outcome {
            PairingOutcome::Rejected => Self::Rejected,
            PairingOutcome::PlacedNoPair => Self::PlacedNoPair,
            PairingOutcome::PlacedPairSkipped => Self::PlacedPairSkipped,
            PairingOutcome::PairPlaced { down, at } => Self::PairPlaced {
                down: TerrainKeyNet::new((*down).to_string()),
                at:   EditorCellLevelNet::from_slot(at),
            },
        }
    }
}
