//! The Theme form's own fields, one arm per control its field stack draws.

use serde::{Deserialize, Serialize};

use super::draft_name::EditorDraftNameNet;

/// One field of the Theme draft, carrying the value it is set to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(in crate::mcp) enum ThemeFieldNet {
    /// The draft's display name.
    Name(EditorDraftNameNet),
}
