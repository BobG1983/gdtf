//! The **attachment registry** — the key→spec map the folder loader builds and the battle
//! setup resolves a weapon's [`attachments`](crate::weapon::WeaponSpec::attachments) keys
//! against (GTW-549, PHASE 1; GTW-558 re-homed into the attachment MECHANICS module), the
//! attachment mirror of the [`MeleeWeaponRegistry`](crate::weapon::MeleeWeaponRegistry) /
//! ranged [`WeaponRegistry`](crate::weapon::WeaponRegistry).

use bevy::{platform::collections::HashMap, prelude::Resource};

use super::{AttachmentName, AttachmentSpec};

/// The **attachment registry** — a key→spec map the folder loader builds from
/// `assets/content/attachments/*.attachment.ron` (keyed by each file's stem) and the battle
/// setup resolves a weapon's
/// [`attachments`](crate::weapon::WeaponSpec::attachments) keys against (GTW-549),
/// the attachment mirror of the [`MeleeWeaponRegistry`](crate::weapon::MeleeWeaponRegistry).
///
/// A named newtype [`Resource`] over a [`HashMap`]`<`[`AttachmentName`]`,
/// `[`AttachmentSpec`]`>` (no-bare-types: a registry is a domain value, not a bare
/// `HashMap`). The sim OWNS the attachment model, so the type lives here; the app's `Load`
/// flow POPULATES it from the loaded `assets/content/attachments/*.attachment.ron` folder
/// (keyed by each file's stem) and inserts it as a resource. It holds the specs BY VALUE
/// ([`AttachmentSpec`] is `Clone`), so they survive even if the loaded-folder asset handle
/// is dropped.
///
/// Private inner with small accessors (the registry answers an attachment LOOKUP, not a
/// raw-map question — so no derived [`Deref`](bevy::prelude::Deref)). Setup resolves a
/// weapon's authored slot keys through [`spec`](AttachmentRegistry::spec) at setup time. A
/// missing key fails closed (nothing applied), the fail-safe the melee/weapon registries
/// share.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct AttachmentRegistry(HashMap<AttachmentName, AttachmentSpec>);

impl AttachmentRegistry {
    /// Build an attachment registry from a `(name, spec)` iterator — the shape the folder
    /// loader (and a test) keys by filename stem.
    #[must_use]
    pub fn new(items: impl IntoIterator<Item = (AttachmentName, AttachmentSpec)>) -> Self {
        Self(items.into_iter().collect())
    }

    /// Insert one attachment spec under its [`AttachmentName`] key, returning the previous
    /// spec at that key (if any) — the per-file insert the folder loader calls as it
    /// iterates the loaded folder.
    pub fn insert(&mut self, name: AttachmentName, spec: AttachmentSpec) -> Option<AttachmentSpec> {
        self.0.insert(name, spec)
    }

    /// Look up the [`AttachmentSpec`] for an attachment KEY, or [`None`] if no attachment
    /// file with that stem was loaded — the setup-time resolution the battle reads to fold
    /// each authored slot's effects.
    #[must_use]
    pub fn spec(&self, name: &AttachmentName) -> Option<&AttachmentSpec> {
        self.0.get(name)
    }

    /// How many attachments the registry holds — the count the folder-load test asserts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the registry holds no attachments.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterate over every [`AttachmentName`] key in the registry — for editor dropdown
    /// enumeration (the [`WeaponRegistry::keys`](crate::weapon::WeaponRegistry) precedent),
    /// so an attachment-selector UI can list all loaded attachments without exposing the
    /// inner map.
    ///
    /// [`HashMap`] iteration order is unspecified; callers that need a stable order must
    /// collect and sort.
    pub fn keys(&self) -> impl Iterator<Item = &AttachmentName> {
        self.0.keys()
    }

    /// Iterate over every `(`[`AttachmentName`]`,` [`AttachmentSpec`]`)` pair in the
    /// registry — for editor dropdown enumeration (the melee/ranged registry precedent).
    ///
    /// [`HashMap`] iteration order is unspecified; callers that need a stable order must
    /// collect and sort by name.
    pub fn iter(&self) -> impl Iterator<Item = (&AttachmentName, &AttachmentSpec)> {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a AttachmentRegistry {
    type Item = (&'a AttachmentName, &'a AttachmentSpec);
    type IntoIter = bevy::platform::collections::hash_map::Iter<'a, AttachmentName, AttachmentSpec>;

    /// Iterate over `(`[`AttachmentName`]`,` [`AttachmentSpec`]`)` pairs via the
    /// [`IntoIterator`] trait — satisfies the `iter_without_into_iter` pedantic lint that
    /// requires a matching trait impl alongside an inherent `iter(&self)`. Delegates to the
    /// inner [`HashMap`]'s owned iterator; order is unspecified.
    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}
