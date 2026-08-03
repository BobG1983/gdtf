//! Folder handle resource for a content family.

use core::marker::PhantomData;

use bevy::{asset::LoadedFolder, prelude::*};

use crate::family::def::ContentFamily;

/// Handle to the loaded folder for family `F`.
#[derive(Resource, Deref)]
pub struct ContentFolderHandle<F: ContentFamily>(#[deref] Handle<LoadedFolder>, PhantomData<F>);

impl<F: ContentFamily> ContentFolderHandle<F> {
    /// Wrap a folder handle.
    #[must_use]
    pub const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle, PhantomData)
    }
}

impl<F: ContentFamily> core::fmt::Debug for ContentFolderHandle<F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("ContentFolderHandle").field(&self.0).finish()
    }
}
