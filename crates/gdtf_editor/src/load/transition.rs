//! `Update` (during [`EditorState::Load`](crate::EditorState)): leave `Load` for
//! `Editing` once the theme + all registries are present.

use bevy::prelude::*;
use gdtf_battle_presenter::TileRoles;
use gdtf_battle_sim::{
    armor::ArmorRegistry, level::UuidThemeRegistry, terrain::def::TerrainDefRegistry,
    weapon::WeaponRegistry,
};
use gdtf_ui::theme::GdtfTheme;

use crate::EditorState;

/// Transitions [`EditorState::Load`] → [`EditorState::Editing`] once the [`GdtfTheme`], the
/// legacy [`WeaponRegistry`] / [`ArmorRegistry`], the (GTW-487) NEW [`TerrainDefRegistry`] /
/// [`UuidThemeRegistry`], and the (GTW-495) [`TileRoles`] are all inserted (the resolve pass
/// inserts each on success OR on its const-default / empty failure fallback, so this is reached
/// even on a bad asset folder — the no-strand guarantee). Takes them as `Option<Res<…>>` and
/// only sets the next state when all are present, so it never panics on an absent resource
/// (`bevy-traps.md` #1).
pub(crate) fn transition_to_editing(
    theme: Option<Res<GdtfTheme>>,
    weapons: Option<Res<WeaponRegistry>>,
    armor: Option<Res<ArmorRegistry>>,
    terrain_defs: Option<Res<TerrainDefRegistry>>,
    theme_defs: Option<Res<UuidThemeRegistry>>,
    tile_roles: Option<Res<TileRoles>>,
    mut next: ResMut<NextState<EditorState>>,
) {
    if theme.is_some()
        && weapons.is_some()
        && armor.is_some()
        && terrain_defs.is_some()
        && theme_defs.is_some()
        && tile_roles.is_some()
    {
        next.set(EditorState::Editing);
    }
}
