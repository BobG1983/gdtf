//! The shell's mode-agnostic **chrome** — the top-bar mode tabs + global theme
//! `ComboBox` and the bottom status line (split out of `shell.rs` at the GTW-636 seam:
//! the chrome changes when the Workbench's frame does, the shell when the per-mode
//! panel layout does).

use bevy_egui::egui;
use gdtf_battle_sim::level::{ThemeUuid, UuidThemeRegistry};

use crate::{egui_shell::theme_combo::ThemeOption, mode::EditorMode, session::MapEditorSession};

/// The placeholder label shown when no theme is selected (the [`ThemeUuid::nil`] sentinel) — the
/// status line's nil-theme text and the `ComboBox`'s empty preview.
const NO_THEME: &str = "—";

/// Draw ONE mode tab per Workbench mode as egui
/// [`selectable_value`](egui::Ui::selectable_value)s over the [`EditorMode`] resource — a click
/// sets the mode in place (the `1`–`9` + `0` number hotkeys do the same via
/// [`mode_hotkeys`](crate::mode::mode_hotkeys)). The tab ORDER is [`EditorMode::TAB_ORDER`] and
/// each label is [`EditorMode::tab_label`], so a new mode variant joins the bar with no edit
/// here (the roster was hand-enumerated in this doc until GTW-670 — it had already drifted).
pub(super) fn mode_tabs(ui: &mut egui::Ui, mode: &mut EditorMode) {
    for option in EditorMode::TAB_ORDER {
        ui.selectable_value(mode, option, option.tab_label());
    }
}

/// Draw the global theme [`ComboBox`](egui::ComboBox) — its options are the sorted
/// [`theme_options`](crate::egui_shell::theme_combo::theme_options), the currently-selected
/// theme's display name is the preview, and choosing a
/// row folds the selection into the session exactly as the old `apply_theme_selection` did
/// (resolving the chosen theme's default-floor from the registry, then calling
/// [`MapEditorSession::select_theme`]).
pub(super) fn theme_combo_box(
    ui: &mut egui::Ui,
    options: &[ThemeOption],
    themes: Option<&UuidThemeRegistry>,
    session: &mut MapEditorSession,
) {
    let selected = session.theme();
    let preview = theme_label(selected, themes);
    ui.label("Theme:");
    egui::ComboBox::from_id_salt("editor_theme_combo")
        .selected_text(preview)
        .show_ui(ui, |ui| {
            for option in options {
                let key = option.key();
                if ui
                    .selectable_label(key == selected, option.label())
                    .clicked()
                {
                    let default_floor = themes.and_then(|themes| themes.default_floor(&key));
                    session.select_theme(key, default_floor);
                }
            }
        });
}

/// The status line text — `"Mode: {LABEL}  |  Theme: {name}"`, with a nil / unknown theme shown as
/// the [`NO_THEME`] placeholder (never a panic). Reuses [`EditorMode::tab_label`] and resolves the
/// session theme's display name from the registry — the verbatim text the old `refresh_status_bar`
/// produced.
pub(super) fn status_line(
    mode: EditorMode,
    session: &MapEditorSession,
    themes: Option<&UuidThemeRegistry>,
) -> String {
    let theme_label = theme_label(session.theme(), themes);
    format!("Mode: {}  |  Theme: {theme_label}", mode.tab_label())
}

/// Resolve a theme's display name from the registry, or the [`NO_THEME`] placeholder for the nil
/// sentinel / an unknown / an absent registry (never a panic).
fn theme_label(theme: ThemeUuid, themes: Option<&UuidThemeRegistry>) -> String {
    if theme.is_nil() {
        return NO_THEME.to_owned();
    }
    themes
        .and_then(|themes| themes.def(&theme))
        .map_or_else(|| NO_THEME.to_owned(), |def| (*def.display_name).clone())
}
