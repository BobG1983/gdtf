use gdtf_battle_presenter::{resolve_sprite, view_key_for};
use gdtf_battle_sim::terrain::{
    def::{TerrainDefRegistry, TerrainUuid},
    facing::TerrainFacing,
};
use gdtf_content_families::sprites::{SpriteDef, SpriteDefRegistry};

/// The sprite def a terrain piece draws when it is turned `facing`, resolved through the
/// view its own def names for that facing.
#[must_use]
pub(crate) fn terrain_sprite_def<'a>(
    registry: &TerrainDefRegistry,
    sprites: &'a SpriteDefRegistry,
    key: &TerrainUuid,
    facing: TerrainFacing,
) -> Option<&'a SpriteDef> {
    let def = registry.def(key)?;
    let sprite = view_key_for(def, facing, None, None)?;
    resolve_sprite(sprites, sprite)
}
