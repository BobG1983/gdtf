//! Attachment content key.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

/// Name of an attachment in content / weapon loadouts.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AttachmentName(String);

impl AttachmentName {
    /// Wrap a name.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}
