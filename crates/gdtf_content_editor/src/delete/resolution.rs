//! How a delete drops the reference to the record from every record that names it.

use std::path::PathBuf;

use bevy::prelude::World;
use gdtf_assets::ContentMemberKey;

use crate::net_qa::EditorQaAssetsRoot;

/// What a drop resolution did to the records that referred to the deleted key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DroppedReferences {
    /// Nothing was rewritten, so the reference stands and the delete is refused.
    Nothing,
    /// Every referring record this drop rewrites was written back.
    Rewritten,
    /// A rewrite could not be written, so the delete must not go through.
    Failed,
}

/// Rewrites every record referring to a key so that it refers to it no longer.
pub type DropReferences =
    Box<dyn Fn(&mut World, &ContentMemberKey) -> DroppedReferences + Send + Sync + 'static>;

/// Rewrites every record referring to a key so that it refers to the replacement instead.
///
/// The first key is the record being deleted, the second the replacement the author chose.
pub type ReplaceReferences = Box<
    dyn Fn(&mut World, &ContentMemberKey, &ContentMemberKey) -> DroppedReferences
        + Send
        + Sync
        + 'static,
>;

/// What the records of one referring family do when the record they name goes.
pub enum ReferenceResolution {
    /// The reference goes, and the referring record keeps everything else.
    Drop(DropReferences),
    /// The reference is pointed at the replacement the author chose.
    Replace(ReplaceReferences),
}

/// The first thing a chosen replacement cannot stand in for, if there is one.
///
/// The first key is the record being deleted, the second the replacement.
pub type ReplacementCheck = Box<
    dyn Fn(&World, &ContentMemberKey, &ContentMemberKey) -> Option<ContentMemberKey>
        + Send
        + Sync
        + 'static,
>;

// The assets root a delete writes its rewritten referrers under.
pub(super) fn delete_assets_root(world: &World) -> Option<PathBuf> {
    world
        .get_resource::<EditorQaAssetsRoot>()
        .map(|root| (**root).clone())
}
