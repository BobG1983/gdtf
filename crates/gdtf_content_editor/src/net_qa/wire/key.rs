//! Registry keys, content names, and written paths on the wire.

use std::path::Path;

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

/// A registry key a client asks the editor to load.
#[derive(Deref, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct EditorKeyNet(String);

impl EditorKeyNet {
    /// Wrap a key string.
    #[must_use]
    pub(in crate::net_qa) const fn new(key: String) -> Self {
        Self(key)
    }
}

/// The name a Prefab save writes its file under.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct EditorContentNameNet(String);

/// The file a save wrote, as text a client can compare.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(in crate::net_qa) struct SavedPathNet(String);

impl SavedPathNet {
    /// Mirror the path a save reported.
    #[must_use]
    pub(in crate::net_qa) fn from_path(path: &Path) -> Self {
        Self(path.display().to_string())
    }
}
