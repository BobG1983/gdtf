//! The Gang form's own fields, one arm per control its roster widgets draw.

use serde::{Deserialize, Serialize};

use super::draft_name::EditorDraftNameNet;
use crate::net_qa::wire::{
    gang::{GangAttributeNet, GangAttributeValueNet},
    key::EditorKeyNet,
    list::EditorListIndexNet,
};

/// One field of the Gang draft, carrying the value it is set to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(in crate::net_qa) enum GangFieldNet {
    /// The draft's own name.
    Name(EditorDraftNameNet),
    /// One member's display name.
    MemberName {
        /// Which member.
        index: EditorListIndexNet,
        /// The name it is set to.
        name:  EditorDraftNameNet,
    },
    /// One member's attribute.
    MemberAttribute {
        /// Which member.
        index:     EditorListIndexNet,
        /// Which of the member's eight attributes.
        attribute: GangAttributeNet,
        /// The value it is set to.
        value:     GangAttributeValueNet,
    },
    /// One member's primary weapon key.
    MemberWeapon {
        /// Which member.
        index: EditorListIndexNet,
        /// The weapon registry key it is set to.
        key:   EditorKeyNet,
    },
    /// One member's armor key.
    MemberArmor {
        /// Which member.
        index: EditorListIndexNet,
        /// The armor registry key it is set to.
        key:   EditorKeyNet,
    },
    /// One member's melee weapon key, set or cleared to the fists default.
    MemberMeleeWeapon {
        /// Which member.
        index: EditorListIndexNet,
        /// The melee registry key it is set to, or none for the fists default.
        key:   Option<EditorKeyNet>,
    },
}
