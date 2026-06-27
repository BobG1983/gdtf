//! `Update` (during [`EditorState::Load`](crate::EditorState)): leave `Load` for
//! `Editing` once the theme + all three registries are present.

use bevy::prelude::*;
use gdtf_battle_sim::{armor::ArmorRegistry, level::ThemeCatalogRegistry, weapon::WeaponRegistry};
use gdtf_ui::theme::GdtfTheme;

use crate::EditorState;

/// Transitions [`EditorState::Load`] → [`EditorState::Editing`] once the
/// [`GdtfTheme`] and the [`WeaponRegistry`] / [`ArmorRegistry`] / [`ThemeCatalogRegistry`]
/// are all inserted (the resolve pass inserts each on success OR on its const-default
/// failure fallback, so this is reached even on a bad asset folder — the no-strand
/// guarantee). Takes the four as `Option<Res<…>>` and only sets the next state when all
/// four are present, so it never panics on an absent resource (`bevy-traps.md` #1).
pub(crate) fn transition_to_editing(
    theme: Option<Res<GdtfTheme>>,
    weapons: Option<Res<WeaponRegistry>>,
    armor: Option<Res<ArmorRegistry>>,
    themes: Option<Res<ThemeCatalogRegistry>>,
    mut next: ResMut<NextState<EditorState>>,
) {
    if theme.is_some() && weapons.is_some() && armor.is_some() && themes.is_some() {
        next.set(EditorState::Editing);
    }
}
