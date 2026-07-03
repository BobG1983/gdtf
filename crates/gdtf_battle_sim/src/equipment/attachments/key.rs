//! The **attachment-item key** — the [`AttachmentName`] a weapon references in its
//! [`attachments`](crate::weapon::WeaponSpec::attachments) and the
//! [`AttachmentRegistry`](super::AttachmentRegistry) keys each loaded item by (GTW-549,
//! PHASE 1; GTW-558 re-homed into the attachment MECHANICS module). The attachment mirror of
//! the ranged [`WeaponName`](crate::weapon::WeaponName).

use bevy::prelude::Deref;
use serde::Deserialize;

/// An **attachment item's key** — its stable identity, the filename stem of its
/// `assets/content/attachments/<key>.attachment.ron` file (GTW-549). A weapon references
/// an attachment BY this key in its
/// [`attachments`](crate::weapon::WeaponSpec::attachments) (authored once,
/// referenced by many), and the [`AttachmentRegistry`](super::AttachmentRegistry) keys each
/// loaded [`AttachmentSpec`](super::AttachmentSpec) by it.
///
/// An attachment-identity newtype over [`String`] (no-bare-types: a key is a domain value),
/// mirroring [`WeaponName`](crate::weapon::WeaponName). Private inner + derived [`Deref`];
/// `#[serde(transparent)]` so a weapon's `attachments:` list authors bare RON strings
/// (`["scoped_sight"]`). `Hash` + `Eq` so it keys the registry [`HashMap`](bevy::platform::collections::HashMap).
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct AttachmentName(String);

impl AttachmentName {
    /// Build an attachment-item key from its stem string (the file stem, or an authored
    /// slot reference).
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}
