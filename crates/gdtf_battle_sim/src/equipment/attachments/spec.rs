//! Authored attachment definition.

use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

use super::AttachmentSlot;
use crate::{effects::attachments::AttachmentEffect, weapon::WeaponName};

/// One attachment from content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TypePath)]
pub struct AttachmentSpec {
    /// Display name.
    pub display_name: WeaponName,
    /// Slot this attachment occupies.
    pub slot:         AttachmentSlot,
    /// Effects applied when fitted.
    #[serde(default)]
    pub effects:      Vec<AttachmentEffect>,
}
