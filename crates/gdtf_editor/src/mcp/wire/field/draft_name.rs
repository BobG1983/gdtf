//! The name field every form carries, whichever form a write names.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

/// The name a form's own name field holds, for whichever form named it.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::mcp) struct EditorDraftNameNet(String);

impl EditorDraftNameNet {
    /// Wrap a name a client sent or a draft holds.
    pub(in crate::mcp) fn new(name: &str) -> Self {
        Self(name.to_owned())
    }
}
