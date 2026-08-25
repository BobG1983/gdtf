//! Attachment keys a weapon ships pre-fitted with.

use bevy::prelude::Deref;
use serde::{Deserialize, Serialize};

use super::AttachmentName;

/// Fitted attachment ITEM keys a weapon declares.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FittedAttachments(Vec<AttachmentName>);

impl FittedAttachments {
    /// From authored keys.
    #[must_use]
    pub const fn new(keys: Vec<AttachmentName>) -> Self {
        Self(keys)
    }

    /// Append a key.
    pub fn add(&mut self, key: AttachmentName) {
        self.0.push(key);
    }

    /// Remove a key by index. Returns whether one was removed.
    pub fn remove(&mut self, index: usize) -> bool {
        if index < self.0.len() {
            self.0.remove(index);
            true
        } else {
            false
        }
    }

    /// Rewrite a key by index. Returns whether one was written.
    pub fn set(&mut self, index: usize, key: AttachmentName) -> bool {
        match self.0.get_mut(index) {
            Some(slot) => {
                *slot = key;
                true
            }
            None => false,
        }
    }
}
