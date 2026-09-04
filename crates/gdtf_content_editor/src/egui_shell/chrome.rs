use bevy_egui::egui;
use gdtf_battle_sim::level::{ThemeUuid, UuidThemeRegistry};

use crate::{egui_shell::theme_combo::ThemeOption, mode::EditorMode, session::MapEditorSession};

const NO_THEME: &str = "—";

pub(super) fn mode_tabs(ui: &mut egui::Ui, mode: &mut EditorMode) {
    for option in EditorMode::TAB_ORDER {
        ui.selectable_value(mode, option, option.tab_label());
    }
}

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

/// The top tab bar and the bottom status line, which every mode draws the same way.
pub(super) fn chrome_bars(
    viewport_ui: &mut egui::Ui,
    mode: &mut EditorMode,
    session: &mut MapEditorSession,
    options: &[ThemeOption],
    themes: Option<&UuidThemeRegistry>,
) {
    egui::Panel::top("editor_top_bar").show(viewport_ui, |ui| {
        ui.horizontal(|ui| {
            mode_tabs(ui, mode);
            ui.separator();
            theme_combo_box(ui, options, themes, session);
        });
    });
    egui::Panel::bottom("editor_status_bar").show(viewport_ui, |ui| {
        ui.label(status_line(*mode, session, themes));
    });
}

pub(super) fn status_line(
    mode: EditorMode,
    session: &MapEditorSession,
    themes: Option<&UuidThemeRegistry>,
) -> String {
    let theme_label = theme_label(session.theme(), themes);
    format!("Mode: {}  |  Theme: {theme_label}", mode.tab_label())
}

fn theme_label(theme: ThemeUuid, themes: Option<&UuidThemeRegistry>) -> String {
    if *theme.is_nil() {
        return NO_THEME.to_owned();
    }
    themes
        .and_then(|themes| themes.def(&theme))
        .map_or_else(|| NO_THEME.to_owned(), |def| (*def.display_name).clone())
}
