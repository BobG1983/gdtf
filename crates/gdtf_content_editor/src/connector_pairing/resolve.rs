use gdtf_battle_presenter::TileRole;
use gdtf_battle_sim::terrain::def::{TerrainDefRegistry, TerrainUuid};

#[must_use]
pub fn is_up_connector(registry: &TerrainDefRegistry, tile: &TerrainUuid) -> bool {
    graphic_name(registry, tile)
        .and_then(TileRole::from_key)
        .is_some_and(TileRole::is_up_connector)
}

#[must_use]
pub fn resolve_down_counterpart(
    registry: &TerrainDefRegistry,
    up_tile: &TerrainUuid,
) -> Option<TerrainUuid> {
    let up_role = TileRole::from_key(graphic_name(registry, up_tile)?)?;
    if !up_role.is_up_connector() {
        return None;
    }
    let down_name = up_role.counterpart()?.as_key();
    registry
        .defs()
        .find(|(_, def)| graphic_key_str(def) == down_name)
        .map(|(key, _)| *key)
}

#[must_use]
fn graphic_name<'a>(registry: &'a TerrainDefRegistry, tile: &TerrainUuid) -> Option<&'a str> {
    registry.def(tile).map(graphic_key_str)
}

#[must_use]
fn graphic_key_str(def: &gdtf_battle_sim::terrain::def::TerrainDef) -> &str {
    use gdtf_battle_sim::terrain::def::TerrainPresenterKind;
    match &def.presenter_kind {
        TerrainPresenterKind::Wall { graphic_name }
        | TerrainPresenterKind::Cover { graphic_name }
        | TerrainPresenterKind::Emplacement { graphic_name }
        | TerrainPresenterKind::Slab { graphic_name, .. } => graphic_name,
    }
}
