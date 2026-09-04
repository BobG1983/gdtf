//! Where each content family member was read from, by the key it was inserted under.

use core::marker::PhantomData;
use std::{collections::HashMap, path::PathBuf};

use bevy::{
    ecs::{
        resource::Resource,
        system::{ResMut, SystemParam},
    },
    prelude::Deref,
};

use crate::family::def::{ContentFamily, ContentMemberKey};

/// Asset-root-relative path a content family member was read from.
#[derive(Deref, Debug, Clone, PartialEq, Eq)]
pub struct ContentSourcePath(PathBuf);

impl ContentSourcePath {
    /// Wrap an asset-root-relative path.
    #[must_use]
    pub const fn new(path: PathBuf) -> Self {
        Self(path)
    }
}

/// The file every member of family `F` was read from, by the key it was inserted under.
#[derive(Resource)]
pub struct ContentSourcePaths<F: ContentFamily>(
    HashMap<ContentMemberKey, ContentSourcePath>,
    PhantomData<F>,
);

impl<F: ContentFamily> ContentSourcePaths<F> {
    /// Collect the key and path pairs a registry build hands over.
    #[must_use]
    pub fn new(paths: impl IntoIterator<Item = (ContentMemberKey, ContentSourcePath)>) -> Self {
        Self(paths.into_iter().collect(), PhantomData)
    }

    /// The file the member under `key` was read from.
    #[must_use]
    pub fn path(&self, key: &ContentMemberKey) -> Option<&ContentSourcePath> {
        self.0.get(key)
    }

    /// Move the recorded file from one key to another, for a rewrite that re-keys a member.
    pub fn rekey(&mut self, from: &ContentMemberKey, to: ContentMemberKey) {
        if let Some(path) = self.0.remove(from) {
            self.0.insert(to, path);
        }
    }
}

impl<F: ContentFamily> Default for ContentSourcePaths<F> {
    fn default() -> Self {
        Self(HashMap::new(), PhantomData)
    }
}

impl<F: ContentFamily> core::fmt::Debug for ContentSourcePaths<F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("ContentSourcePaths").field(&self.0).finish()
    }
}

/// The pair family `F` publishes together: its registry and its source paths.
///
/// A rebuild replaces both, so nothing borrows one without the other.
#[derive(SystemParam)]
pub struct PublishedFamily<'w, F: ContentFamily> {
    /// The registry, absent until the family resolves.
    pub registry: Option<ResMut<'w, <F as ContentFamily>::Registry>>,
    /// Where each member was read from, absent until the family resolves.
    pub sources:  Option<ResMut<'w, ContentSourcePaths<F>>>,
}
