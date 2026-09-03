//! Named attachment specs from content.

use bevy::prelude::Resource;

use super::{AttachmentName, AttachmentSpec};
use crate::registry::Registry;

/// All loaded attachment specs.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct AttachmentRegistry(Registry<AttachmentName, AttachmentSpec>);

impl AttachmentRegistry {
    /// Build from name→spec pairs.
    #[must_use]
    pub fn new(items: impl IntoIterator<Item = (AttachmentName, AttachmentSpec)>) -> Self {
        Self(Registry::new(items))
    }

    /// Insert or replace a spec.
    pub fn insert(&mut self, name: AttachmentName, spec: AttachmentSpec) -> Option<AttachmentSpec> {
        self.0.insert(name, spec)
    }

    /// Take the spec under `name` out; answers it if it was there.
    pub fn remove(&mut self, name: &AttachmentName) -> Option<AttachmentSpec> {
        self.0.remove(name)
    }

    /// Lookup by name.
    #[must_use]
    pub fn spec(&self, name: &AttachmentName) -> Option<&AttachmentSpec> {
        self.0.get(name)
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// True when empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterate names.
    pub fn keys(&self) -> impl Iterator<Item = &AttachmentName> {
        self.0.keys()
    }

    /// Iterate name→spec pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&AttachmentName, &AttachmentSpec)> {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a AttachmentRegistry {
    type Item = (&'a AttachmentName, &'a AttachmentSpec);
    type IntoIter = bevy::platform::collections::hash_map::Iter<'a, AttachmentName, AttachmentSpec>;

    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}
