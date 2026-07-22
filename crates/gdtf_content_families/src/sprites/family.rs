//! The sprite-defs [`ContentFamily`] marker — the GTW-663 binding of the
//! generic folder→registry machinery to the in-crate sprite model.

use gdtf_assets::{ContentFamily, ContentFileStem};

use super::{
    def::SpriteDef,
    registry::{SpriteDefRegistry, SpriteName},
};

/// The sprite-defs family: `assets/content/sprites/*.spritedef.ron` → the
/// stem-keyed [`SpriteDefRegistry`] a terrain def's `graphic_name` foreign
/// key resolves against (GTW-663; the renderer consumes it in GTW-665).
///
/// STEM-KEYED: `floor.spritedef.ron` keys `floor` (the [`SpriteName`] a
/// `graphic_name` references). The dedicated `spritedef.ron` compound
/// extension is claimed by NO other loader — the same-suffixed OLD role
/// tables under `assets/sprites/` are single-file TYPED loads, untouched by
/// the untyped folder dispatch (see the [module doc](super)).
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
        // Stem-keyed: a handle with no resolvable path/stem is skipped
        // defensively (it would carry no usable key).
        let Some(stem) = stem else { return };
        registry.insert(SpriteName::new(stem.into_inner()), def.clone());
    }
}
