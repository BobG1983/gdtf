//! Prefab name field and debug-only save button.
use bevy_egui::egui;
use gdtf_battle_presenter::{ContextDepth, IsolateView, ViewMode};
use gdtf_battle_sim::level::{Prefab, PrefabRegistry, UuidThemeRegistry};

use crate::{
    canvas::CurrentEditLevel,
    editor_map::EditorMap,
    egui_shell::prefab::{
        level_rail::{self, RailCtx, RailUiState},
        size_fields,
    },
    open::{open_prefab, prefab_candidates},
    save_record::LastSaveRecord,
    session::MapEditorSession,
};

/// The id salt the open picker's combo carries, which its draw test derives the popup id from.
pub(crate) const PREFAB_PICKER_SALT: &str = "prefab_open_picker";

/// The prefab the controls panel edits: its painted cells, its session, its active storey, and
/// the authored prefabs its open picker lists.
pub(crate) struct EditedPrefab<'a> {
    pub(crate) map:        &'a mut EditorMap,
    pub(crate) session:    &'a mut MapEditorSession,
    pub(crate) edit_level: &'a mut CurrentEditLevel,
    pub(crate) prefabs:    Option<&'a PrefabRegistry>,
}

/// The storey visibility toggles the controls panel drives.
pub(crate) struct StoreyToggles<'a> {
    pub(crate) view:    &'a mut ViewMode,
    pub(crate) isolate: &'a mut IsolateView,
}

/// The terrain and theme definitions the prefab's cells resolve against.
#[derive(Clone, Copy)]
pub(crate) struct TerrainLibrary<'a> {
    pub(crate) terrain: Option<&'a gdtf_battle_sim::terrain::def::TerrainDefRegistry>,
    pub(crate) themes:  Option<&'a UuidThemeRegistry>,
}

pub(crate) fn controls_panel(
    ui: &mut egui::Ui,
    prefab: EditedPrefab<'_>,
    storeys: StoreyToggles<'_>,
    library: TerrainLibrary<'_>,
    rail_state: &mut RailUiState,
    save_name: &mut String,
    last_save: &mut LastSaveRecord,
) {
    let EditedPrefab {
        map,
        session,
        edit_level,
        prefabs,
    } = prefab;
    let StoreyToggles { view, isolate } = storeys;

    ui.heading("Prefab");
    ui.separator();

    open_picker(ui, session, map, edit_level, prefabs, library.themes);
    ui.separator();
    size_fields::size_fields(ui, session, edit_level);
    ui.separator();
    level_rail::level_rail(
        ui,
        &mut RailCtx {
            map: &*map,
            session,
            edit_level,
            registry: library.terrain,
            state: rail_state,
        },
    );
    ui.separator();
    view_toggle(ui, view);
    isolate_toggle(ui, isolate);

    #[cfg(debug_assertions)]
    {
        ui.separator();
        save_control(ui, &*map, session, library, save_name, last_save);
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = save_name;
        let _ = last_save;
    }
}

/// The combo of authored prefabs; a clicked row loads that prefab onto the canvas.
///
/// Drawn on the panel's own `ui`, because the combo's button id comes from `ui.id()`.
fn open_picker(
    ui: &mut egui::Ui,
    session: &mut MapEditorSession,
    map: &mut EditorMap,
    edit_level: &mut CurrentEditLevel,
    prefabs: Option<&PrefabRegistry>,
    themes: Option<&UuidThemeRegistry>,
) {
    ui.label("Open prefab");
    let (Some(prefabs), Some(themes)) = (prefabs, themes) else {
        ui.label("(loading…)");
        return;
    };
    let candidates = prefab_candidates(prefabs);
    let mut chosen: Option<&Prefab> = None;
    let mut clicked = false;
    egui::ComboBox::from_id_salt(PREFAB_PICKER_SALT)
        .selected_text("(select…)")
        .show_ui(ui, |ui| {
            for (prefab, label) in &candidates {
                if ui.selectable_label(false, label).clicked() {
                    chosen = Some(prefab);
                    clicked = true;
                }
            }
        });
    if clicked && let Some(prefab) = chosen {
        open_prefab(session, map, edit_level, themes, prefab.spec());
    }
}

fn view_toggle(ui: &mut egui::Ui, view: &mut ViewMode) {
    ui.label("Storey view");
    ui.horizontal(|ui| {
        let label = match *view {
            ViewMode::DownToActive => "Down-to-active ▸ Full view",
            ViewMode::FullView => "Full view ▸ Down-to-active",
        };
        if ui.button(label).clicked() {
            *view = view.toggled();
        }
    });
    ui.label(match *view {
        ViewMode::DownToActive => "Drawing storeys 0..=active (F to toggle)",
        ViewMode::FullView => "Drawing ALL storeys (F to toggle)",
    });
}

fn isolate_toggle(ui: &mut egui::Ui, isolate: &mut IsolateView) {
    let mut on = matches!(*isolate, IsolateView::On(_));
    if ui.checkbox(&mut on, "Isolate (active + 1 below)").changed() {
        *isolate = if on {
            IsolateView::On(ContextDepth::new(1))
        } else {
            IsolateView::Off
        };
    }
    if matches!(*isolate, IsolateView::On(_)) {
        ui.label("Isolate wins over the storey view above");
    }
}

/// Debug-only — the whole prefab save path is `#[cfg(debug_assertions)]`.
#[cfg(debug_assertions)]
fn save_control(
    ui: &mut egui::Ui,
    map: &EditorMap,
    session: &MapEditorSession,
    library: TerrainLibrary<'_>,
    save_name: &mut String,
    last_save: &mut LastSaveRecord,
) {
    ui.label("Prefab name");
    ui.text_edit_singleline(save_name);
    if !ui.button("Save prefab").clicked() {
        return;
    }
    let Some(registry) = library.terrain else {
        bevy::log::error!("prefab save: terrain registry not resolved");
        return;
    };
    let theme_display = library
        .themes
        .and_then(|themes| themes.def(&session.theme()))
        .map_or_else(String::new, |def| (*def.display_name).clone());
    let written = crate::save::write_prefab(map, registry, session, &theme_display, save_name);
    match &written {
        Ok(path) => bevy::log::info!("prefab save: wrote prefab to `{}`", path.display()),
        Err(err) => bevy::log::error!("prefab save: {err}"),
    }
    last_save.record(
        crate::EditorMode::Prefab,
        crate::save_record::SaveOutcome::from_result(written),
    );
}
