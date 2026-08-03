use gdtf_assets::{ContentFamily, ContentFileStem};

use super::{
    def::SpriteDef,
    registry::{SpriteDefRegistry, SpriteName},
};

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
    ) {
        let Some(stem) = stem else { return };
        registry.insert(SpriteName::new(stem.into_inner()), def.clone());
    }
}
