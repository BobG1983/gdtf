use gdtf_battle_presenter::resolve_sprite;
use gdtf_battle_sim::terrain::{
    def::{TerrainDef, TerrainDefRegistry, TerrainPresenterKind, TerrainUuid},
    piece::TerrainGraphicKey,
};
use gdtf_content_families::sprites::{SpriteDef, SpriteDefRegistry};

#[must_use]
pub(crate) const fn graphic_key(def: &TerrainDef) -> &TerrainGraphicKey {
    match &def.presenter_kind {
        TerrainPresenterKind::Wall { graphic_name }
        | TerrainPresenterKind::Cover { graphic_name }
        | TerrainPresenterKind::Emplacement { graphic_name }
        | TerrainPresenterKind::Slab { graphic_name, .. } => graphic_name,
    }
}

#[must_use]
pub(crate) fn terrain_sprite_def<'a>(
    registry: &TerrainDefRegistry,
    sprites: &'a SpriteDefRegistry,
    key: &TerrainUuid,
) -> Option<&'a SpriteDef> {
    let def = registry.def(key)?;
    resolve_sprite(sprites, graphic_key(def))
}
