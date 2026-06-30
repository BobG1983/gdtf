//! The Workbench **status bar** refresh (GTW-474) — keeps the bottom status strip's text in
//! sync with the active mode + theme.
//!
//! The status bar ([`EditorStatusBar`](crate::regions::EditorStatusBar)) is a thin
//! [`GlobalZIndex`](bevy::prelude::GlobalZIndex)'d strip with a single mutated
//! [`Text`](bevy::prelude::Text) line ([`StatusText`](crate::regions::StatusText)). This system
//! rewrites it in place (the ui-mutate rule) whenever the mode or the session theme changes, so
//! the author always sees which workflow they are in.

use bevy::prelude::*;
use gdtf_battle_sim::level::UuidThemeRegistry;

use crate::{mode::EditorMode, regions::StatusText, session::MapEditorSession};

/// `Update` (in `Editing`): rewrite the status bar's [`Text`] with the active mode + theme
/// (GTW-474).
///
/// Runs only when the [`EditorMode`] or the [`MapEditorSession`] changed (`is_changed()` covers
/// the first insert so the initial line is set once), then composes a one-line summary —
/// `"Mode: TERRAIN  |  Theme: Industrial Hive"` — resolving the session theme's display name from
/// the [`UuidThemeRegistry`]. Every borrow is optional (the mode + session are state-scoped —
/// bevy-traps #1); a nil / unknown theme shows a placeholder rather than panicking.
pub(crate) fn refresh_status_bar(
    mode: Option<Res<EditorMode>>,
    session: Option<Res<MapEditorSession>>,
    themes: Option<Res<UuidThemeRegistry>>,
    mut status_text: Query<&mut Text, With<StatusText>>,
) {
    let (Some(mode), Some(session)) = (mode, session) else {
        return;
    };
    if !mode.is_changed() && !session.is_changed() {
        return;
    }
    let Ok(mut text) = status_text.single_mut() else {
        return;
    };
    let theme_label = themes
        .as_deref()
        .and_then(|themes| themes.def(&session.theme()))
        .map_or_else(|| "—".to_owned(), |def| (*def.display_name).clone());
    *text = Text::new(format!(
        "Mode: {}  |  Theme: {theme_label}",
        mode.tab_label()
    ));
}
