//! Why an editor write command turned a mode down.

use serde::{Deserialize, Serialize};

/// Why this build will not run a write command against the named mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(in crate::net_qa) enum EditorRefusalNet {
    /// That form draws no New button, so there is no blank-draft constructor it calls.
    NoNewAction,
    /// A blank Theme draft cannot survive the form's own sync, which reloads the session theme.
    ThemeNewIsUndoneBySync,
    /// That form loads nothing by key, so there is no registry entry to fetch.
    NoLoadAction,
    /// Only Prefab carries its own name field, so no other mode accepts a name.
    NameBelongsToPrefabOnly,
    /// Prefab saves under the name its form's own field holds, so the name is required.
    PrefabNeedsAName,
}
