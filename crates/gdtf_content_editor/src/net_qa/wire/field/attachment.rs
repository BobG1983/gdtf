//! The Attachment form's own fields, one arm per control its def panel draws.

use serde::{Deserialize, Serialize};

use super::draft_name::EditorDraftNameNet;
use crate::net_qa::wire::{
    attachment::{AttachmentEffectNet, AttachmentSlotNet},
    list::EditorListIndexNet,
};

/// One field of the Attachment draft, carrying the value it is set to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(in crate::net_qa) enum AttachmentFieldNet {
    /// The draft's name.
    Name(EditorDraftNameNet),
    /// The draft's display name.
    DisplayName(EditorDraftNameNet),
    /// The draft's mounting slot.
    Slot(AttachmentSlotNet),
    /// One authored effect, variant and payload together.
    Effect {
        /// Which effect.
        index:  EditorListIndexNet,
        /// The effect it is set to.
        effect: AttachmentEffectNet,
    },
}
