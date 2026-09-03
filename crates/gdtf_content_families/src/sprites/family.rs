//! Sprite definition content family.

use gdtf_assets::{ContentFamily, ContentFileStem, ContentMemberKey};

use super::{
    def::SpriteDef,
    registry::{SpriteDefRegistry, SpriteName},
};

/// Loads `*.spritedef.ron` files into [`SpriteDefRegistry`].
pub struct SpriteDefsFamily;

impl ContentFamily for SpriteDefsFamily {
    type Spec = SpriteDef;
    type Registry = SpriteDefRegistry;

    const EXTENSION: &'static str = "spritedef.ron";
    const FOLDER: &'static str = "content/sprites";

    fn insert_member(
        registry: &mut SpriteDefRegistry,
        stem: Option<ContentFileStem>,
        def: &SpriteDef,
    ) -> Option<ContentMemberKey> {
        let key = stem?.into_inner();
        registry.insert(SpriteName::new(key.clone()), def.clone());
        Some(ContentMemberKey::new(key))
    }
}
