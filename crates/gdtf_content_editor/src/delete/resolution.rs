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

// The assets root a delete writes its rewritten referrers under.
pub(super) fn delete_assets_root(world: &World) -> Option<PathBuf> {
    world
        .get_resource::<EditorQaAssetsRoot>()
        .map(|root| (**root).clone())
}
