//! What one parked editor `wait` is holding out for, on the wire.

use serde::{Deserialize, Serialize};

use super::content_family::ContentFamilyNet;

/// The one condition a `wait` holds its reply for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum EditorWaitConditionNet {
    /// The content integrity checks have finished their pass.
    ChecksComplete,
    /// The named family's registry has changed since the call was parked.
    RegistryRearmed {
        /// Which registry the wait watches.
        family: ContentFamilyNet,
    },
}
