//! The Injury def form's own fields, one arm per control its panels draw.

use serde::{Deserialize, Serialize};

use super::draft_name::EditorDraftNameNet;
use crate::mcp::wire::{
    injury::{InjuryCategoryNet, InjuryEffectNet, InjuryKeyNet, InjurySeverityNet, InjuryTextNet},
    list::EditorListIndexNet,
};

/// One field of the Injury draft, carrying the value it is set to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(in crate::mcp) enum InjuryFieldNet {
    /// The draft's key.
    Key(InjuryKeyNet),
    /// The draft's display name.
    Name(EditorDraftNameNet),
    /// The draft's category.
    Category(InjuryCategoryNet),
    /// The draft's severity rank.
    Severity(InjurySeverityNet),
    /// The draft's popup text.
    PopupText(InjuryTextNet),
    /// The draft's log text.
    LogText(InjuryTextNet),
    /// The draft's inspect text.
    InspectText(InjuryTextNet),
    /// One authored injury effect, variant and payload together.
    Effect {
        /// Which effect.
        index:  EditorListIndexNet,
        /// The effect it is set to.
        effect: InjuryEffectNet,
    },
}
