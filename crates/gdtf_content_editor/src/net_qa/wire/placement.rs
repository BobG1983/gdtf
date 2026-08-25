//! What the placement rules said about a slot, on the wire.

use serde::{Deserialize, Serialize};

use super::cell::EditorCellLevelNet;
use crate::placement::{IllegalReason, PlacementVerdict};

/// Why the placement rules turned a slot down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum IllegalReasonNet {
    /// The slot sits outside the session's grid.
    OutOfBounds,
    /// A slab there would seal a ladder in or under it.
    SlabSealsLadder,
}

impl IllegalReasonNet {
    /// Mirror the editor's own reason, with no wildcard arm.
    pub(in crate::net_qa) const fn from_reason(reason: IllegalReason) -> Self {
        match reason {
            IllegalReason::OutOfBounds => Self::OutOfBounds,
            IllegalReason::SlabSealsLadder => Self::SlabSealsLadder,
        }
    }
}

/// The verdict the rules returned for the slot the caller asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum PlacementVerdictNet {
    /// The tile may go there, clearing the named slot first if one is named.
    Legal {
        /// The slot the placement clears on its way in.
        auto_clear: Option<EditorCellLevelNet>,
    },
    /// The tile may not go there.
    Illegal(IllegalReasonNet),
}

impl PlacementVerdictNet {
    /// Mirror the editor's own verdict, with no wildcard arm.
    pub(in crate::net_qa) fn from_verdict(verdict: &PlacementVerdict) -> Self {
        match verdict {
            PlacementVerdict::Legal { auto_clear } => Self::Legal {
                auto_clear: auto_clear.map(EditorCellLevelNet::from_slot),
            },
            PlacementVerdict::Illegal(reason) => {
                Self::Illegal(IllegalReasonNet::from_reason(*reason))
            }
        }
    }
}
