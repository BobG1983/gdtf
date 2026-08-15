//! Load a theme: set the session theme the form syncs against, then fill the draft.

use bevy::asset::uuid::Uuid;
use gdtf_battle_sim::level::{ThemeUuid, UuidThemeRegistry};

use super::families::KeyLookup;
use crate::{
    egui_shell::theme_form_ui, net_qa::wire::EditorKeyNet, session::MapEditorSession,
    theme_form::ThemeDraft,
};

// Sorted so a client reading a miss sees the same list every time.
fn known_themes(registry: &UuidThemeRegistry) -> Vec<EditorKeyNet> {
    let mut known: Vec<EditorKeyNet> = registry
        .defs()
        .map(|(key, _)| EditorKeyNet::new((**key).to_string()))
        .collect();
    known.sort();
    known
}

/// Select the theme in the session and load its def into the draft.
pub(in crate::net_qa::commands::write::load) fn load_theme(
    draft: &mut ThemeDraft,
    session: &mut MapEditorSession,
    registry: &UuidThemeRegistry,
    key: &EditorKeyNet,
) -> KeyLookup {
    let Ok(parsed) = Uuid::parse_str(key) else {
        return KeyLookup::NoSuchKey(known_themes(registry));
    };
    let theme = ThemeUuid::new(parsed);
    let Some(def) = registry.def(&theme) else {
        return KeyLookup::NoSuchKey(known_themes(registry));
    };
    session.select_theme(theme, registry.default_floor(&theme));
    theme_form_ui::load_theme_into_form(draft, def);
    KeyLookup::Loaded
}
