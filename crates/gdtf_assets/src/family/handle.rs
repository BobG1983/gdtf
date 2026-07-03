//! The one generic persistent folder handle every content family holds.

use core::marker::PhantomData;

use bevy::{asset::LoadedFolder, prelude::*};

use crate::family::def::ContentFamily;

/// The persistent handle to a content family's ACTIVE loaded folder — the ONE
/// folder-handle newtype every family shares (GTW-570).
///
/// Replaces the per-family pairs of handle newtypes (`WeaponsFolderHandle` +
/// `ActiveWeaponsFolderHandle`, …) that each re-encoded the same wrapper. A
/// marker-parameterized generic newtype satisfies the no-bare-types rule via
/// its rule-4 plumbing carve-out (standing ruling `Q4`, 2026-07-02); rule 5
/// still binds, so the inner [`Handle`] is PRIVATE and read through [`Deref`] /
/// built through [`new`](Self::new).
///
/// Inserted by the generic kick-off
/// ([`kick_off_content_family`](crate::kick_off_content_family)) and NEVER
/// removed: the resolve polls the folder's load state through it, the redrive
/// re-enumerates its member handles to rebuild the registry on a hot edit, and
/// holding it keeps a STRONG reference to every member asset so the
/// file-watcher keeps watching them after `Load` exits.
#[derive(Resource, Deref)]
pub struct ContentFolderHandle<F: ContentFamily>(#[deref] Handle<LoadedFolder>, PhantomData<F>);

impl<F: ContentFamily> ContentFolderHandle<F> {
    /// Wrap the family's in-flight/active folder-load handle.
    #[must_use]
    pub const fn new(handle: Handle<LoadedFolder>) -> Self {
        Self(handle, PhantomData)
    }
}

/// Hand-written so the impl does NOT bound `F: Debug` (family markers are
/// zero-sized tags a derive would needlessly constrain).
impl<F: ContentFamily> core::fmt::Debug for ContentFolderHandle<F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("ContentFolderHandle").field(&self.0).finish()
    }
}
