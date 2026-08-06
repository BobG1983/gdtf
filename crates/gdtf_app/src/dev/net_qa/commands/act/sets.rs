//! Ordering bands the classic act commands run in.

use bevy::prelude::SystemSet;

crate::support_item! {
    /// Where an act command's two systems sit in the frame.
    #[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum ActCommandSystems {
        /// Push the intent after the call is claimed and before the act bus drains.
        Claim,
        /// Claim inside the band, but only once the panel has scanned this frame's offers.
        ContextualClaim,
        /// Answer the parked call once the sim has recorded the frame.
        Settle,
    }
}
