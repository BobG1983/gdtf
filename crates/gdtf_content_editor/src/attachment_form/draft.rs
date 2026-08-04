//! Attachment form draft resource.

use bevy::prelude::*;
use gdtf_battle_sim::{
    equipment::attachments::{AttachmentName, AttachmentSlot, AttachmentSpec},
    weapon::WeaponName,
};

const fn seed_spec() -> AttachmentSpec {
    AttachmentSpec {
        display_name: WeaponName::new(String::new()),
        slot:         AttachmentSlot::Muzzle,
        effects:      Vec::new(),
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AutoloadState {
    Pending,
    Done,
}

/// In-progress attachment being authored.
#[derive(Resource, Clone, PartialEq, Debug)]
pub struct AttachmentDraft {
    name:     String,
    spec:     AttachmentSpec,
    autoload: AutoloadState,
}

impl AttachmentDraft {
    /// Empty draft ready for a new attachment.
    #[must_use]
    pub const fn new_attachment() -> Self {
        Self {
            name:     String::new(),
            spec:     seed_spec(),
            autoload: AutoloadState::Done,
        }
    }

    /// Whether the form should still try to autoload from the registry.
    #[must_use]
    pub const fn autoload_pending(&self) -> bool {
        matches!(self.autoload, AutoloadState::Pending)
    }

    /// Mark autoload complete.
    pub const fn mark_autoloaded(&mut self) {
        self.autoload = AutoloadState::Done;
    }

    /// Load an existing attachment into the draft.
    pub fn load_attachment(&mut self, name: &AttachmentName, spec: &AttachmentSpec) {
        name.as_str().clone_into(&mut self.name);
        self.spec = spec.clone();
        self.autoload = AutoloadState::Done;
    }

    /// Display name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the display name.
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// Full attachment spec.
    #[must_use]
    pub const fn spec(&self) -> &AttachmentSpec {
        &self.spec
    }

    /// Mutable access to the attachment spec.
    pub const fn spec_mut(&mut self) -> &mut AttachmentSpec {
        &mut self.spec
    }
}

impl Default for AttachmentDraft {
    fn default() -> Self {
        Self {
            name:     String::new(),
            spec:     seed_spec(),
            autoload: AutoloadState::Pending,
        }
    }
}
