//! The **attachment authoring spec** — the [`AttachmentSpec`] an
use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

use super::AttachmentSlot;
use crate::{effects::attachments::AttachmentEffect, weapon::WeaponName};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TypePath)]
pub struct AttachmentSpec {
                    pub display_name: WeaponName,
                            pub slot:         AttachmentSlot,
                /// identity. `#[serde(default)]` so a cosmetic attachment that authors no `effects:`
        #[serde(default)]
    pub effects:      Vec<AttachmentEffect>,
}
