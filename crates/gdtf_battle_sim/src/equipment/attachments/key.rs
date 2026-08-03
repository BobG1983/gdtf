use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

/// `#[serde(transparent)]` so a weapon's `attachments:` list authors bare RON strings
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AttachmentName(String);

impl AttachmentName {
            #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}
