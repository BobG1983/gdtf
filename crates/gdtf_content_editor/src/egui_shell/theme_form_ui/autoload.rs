use gdtf_battle_sim::level::{ThemeUuid, UuidThemeDef, UuidThemeRegistry};

use crate::theme_form::ThemeDraft;

pub(crate) fn load_theme_into_form(draft: &mut ThemeDraft, def: &UuidThemeDef) {
    *draft = ThemeDraft::from_parts(
        def.key,
        (*def.display_name).clone(),
        def.terrain.clone(),
        def.default_floor,
    );
}

pub(crate) fn resolve_autoload(
    session_theme: ThemeUuid,
    themes: &UuidThemeRegistry,
) -> Option<&UuidThemeDef> {
    if *session_theme.is_nil() {
        return None;
    }
    themes.def(&session_theme)
}
