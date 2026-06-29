//! `Update` (during [`EditorState::Load`](crate::EditorState)): leave `Load` for
//! `Editing` once the theme + all three registries are present.

use bevy::prelude::*;
use gdtf_battle_sim::{
    armor::ArmorRegistry,
    level::{ThemeCatalogRegistry, UuidThemeRegistry},
    terrain::def::TerrainDefRegistry,
    weapon::WeaponRegistry,
};
use gdtf_ui::theme::GdtfTheme;

use crate::EditorState;

/// Transitions [`EditorState::Load`] → [`EditorState::Editing`] once the [`GdtfTheme`], the
/// legacy [`WeaponRegistry`] / [`ArmorRegistry`] / [`ThemeCatalogRegistry`], and (GTW-487)
/// the NEW [`TerrainDefRegistry`] / [`UuidThemeRegistry`] are all inserted (the resolve pass
/// inserts each on success OR on its const-default / empty failure fallback, so this is
/// reached even on a bad asset folder — the no-strand guarantee). Takes them as
/// `Option<Res<…>>` and only sets the next state when all are present, so it never panics on
/// an absent resource (`bevy-traps.md` #1).
pub(crate) fn transition_to_editing(
    theme: Option<Res<GdtfTheme>>,
    weapons: Option<Res<WeaponRegistry>>,
    armor: Option<Res<ArmorRegistry>>,
    themes: Option<Res<ThemeCatalogRegistry>>,
    terrain_defs: Option<Res<TerrainDefRegistry>>,
    theme_defs: Option<Res<UuidThemeRegistry>>,
    mut next: ResMut<NextState<EditorState>>,
) {
    if theme.is_some()
        && weapons.is_some()
        && armor.is_some()
        && themes.is_some()
        && terrain_defs.is_some()
        && theme_defs.is_some()
    {
        next.set(EditorState::Editing);
    }
}
