use bevy::{app::App, asset::uuid::Uuid};
use gdtf_battle_sim::terrain::def::TerrainUuid;
use gdtf_editor::is_stair;

use crate::{
    support::TestError,
    world::{session, terrain_registry, theme_registry},
};

// A key nothing in the shipped content uses, so no registry can hold it by accident.
const A_KEY_NO_CONTENT_FILE_CARRIES: u128 = 0xF0F0_F0F0_DEAD_BEEF_F0F0_F0F0_DEAD_BEEF;

/// Text no UUID parser accepts, so a helper refuses it before it reaches any registry.
pub(crate) const A_KEY_THAT_IS_NOT_UUID_TEXT: &str = "not-a-uuid";

// The rows `palette_panel` draws: the theme's own terrain that the registry holds a def for.
fn palette_keys(app: &App) -> Result<Vec<TerrainUuid>, TestError> {
    let theme = session(app)?.theme();
    let themes = theme_registry(app)?;
    let terrain = terrain_registry(app)?;
    let Some(listed) = themes.terrain(&theme) else {
        return Err(format!("the seeded theme {theme:?} lists no terrain at all").into());
    };
    let mut keys: Vec<TerrainUuid> = listed
        .iter()
        .copied()
        .filter(|key| terrain.def(key).is_some())
        .collect();
    keys.sort_by_key(|key| (**key).to_string());
    Ok(keys)
}

/// A palette tile that pairs with nothing, so a paint through it writes one cell and no more.
pub(crate) fn a_plain_palette_tile(app: &App) -> Result<TerrainUuid, TestError> {
    let terrain = terrain_registry(app)?;
    let keys = palette_keys(app)?;
    let Some(key) = keys.iter().copied().find(|key| !is_stair(&terrain, key)) else {
        return Err(format!(
            "this case needs a palette tile that is not a staircase, and the seeded theme \
             offers {keys:?}"
        )
        .into());
    };
    Ok(key)
}

/// A palette tile whose paint places a second tile one storey above it.
pub(crate) fn a_stair_palette_tile(app: &App) -> Result<TerrainUuid, TestError> {
    let terrain = terrain_registry(app)?;
    let keys = palette_keys(app)?;
    let Some(key) = keys.iter().copied().find(|key| is_stair(&terrain, key)) else {
        return Err(format!(
            "this case needs the seeded theme to offer a staircase, or the pairing half of \
             the paint reply would never be exercised, and it offers {keys:?}"
        )
        .into());
    };
    Ok(key)
}

/// A terrain the registry holds a def for that the session's theme does not list.
pub(crate) fn a_terrain_off_the_palette(app: &App) -> Result<TerrainUuid, TestError> {
    let listed = palette_keys(app)?;
    let mut held: Vec<TerrainUuid> = terrain_registry(app)?.defs().map(|(key, _)| *key).collect();
    held.sort_by_key(|key| (**key).to_string());
    let Some(key) = held.into_iter().find(|key| !listed.contains(key)) else {
        return Err(
            "this case needs a terrain the registry holds that the seeded theme does not \
                    list, and every loaded def is on that theme's palette"
                .into(),
        );
    };
    Ok(key)
}

/// A key that parses as a UUID and that no terrain registry entry answers to.
pub(crate) fn a_key_no_terrain_holds(app: &App) -> Result<TerrainUuid, TestError> {
    let key = TerrainUuid::new(Uuid::from_u128(A_KEY_NO_CONTENT_FILE_CARRIES));
    if terrain_registry(app)?.def(&key).is_some() {
        return Err(format!(
            "a content file has taken {key:?}, so this case no longer names a key the terrain \
             registry misses"
        )
        .into());
    }
    Ok(key)
}
