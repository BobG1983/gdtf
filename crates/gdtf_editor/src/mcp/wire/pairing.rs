//! What the connector pairing pass did after a tile landed, on the wire.

use serde::{Deserialize, Serialize};

use super::{cell::EditorCellLevelNet, key::TerrainKeyNet};
use crate::connector_pairing::PairingOutcome;

/// Whether a paint placed one tile, two, or none.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::mcp) enum PairingOutcomeNet {
    /// The rules turned the placement down, so nothing was written.
    Rejected,
    /// The tile landed and pairs with nothing.
    PlacedNoPair,
    /// The tile landed, and its second end could not follow.
    PlacedPairSkipped,
    /// The tile landed, and a second tile went in one storey above it.
    PairPlaced {
        /// The tile the pass placed one storey up.
        paired: TerrainKeyNet,
        /// The slot that second tile landed in.
        at:     EditorCellLevelNet,
    },
}

impl PairingOutcomeNet {
    /// Mirror the editor's own outcome, with no wildcard arm.
    pub(in crate::mcp) fn from_outcome(outcome: PairingOutcome) -> Self {
        match outcome {
            PairingOutcome::Rejected => Self::Rejected,
            PairingOutcome::PlacedNoPair => Self::PlacedNoPair,
            PairingOutcome::PlacedPairSkipped => Self::PlacedPairSkipped,
            PairingOutcome::PairPlaced { paired, at } => Self::PairPlaced {
                paired: TerrainKeyNet::new((*paired).to_string()),
                at:     EditorCellLevelNet::from_slot(at),
            },
        }
    }
}
