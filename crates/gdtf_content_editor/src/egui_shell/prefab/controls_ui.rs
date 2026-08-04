//! Prefab name field and debug-only save button.
use bevy_egui::egui;
use gdtf_battle_presenter::{ContextDepth, IsolateView, ViewMode};
use gdtf_battle_sim::level::UuidThemeRegistry;

use crate::{
    canvas::CurrentEditLevel,
    egui_shell::prefab::{
        level_rail::{self, RailCtx, RailUiState},
        size_fields,
    },
    session::MapEditorSession,
};

#[expect(
    clippy::too_many_arguments,
    reason = "the controls panel drives every prefab RIGHT-panel control from the borrows the shell \
              already holds (session + edit level + view mode + the rail state/map/registries + \
              themes + the debug save name); each is a distinct borrow threaded through the shell's \
              panel closure and Bevy's injection cannot reduce them without a wrapper that changes \
              the crate API"
)]
pub(crate) fn controls_panel(
    ui: &mut egui::Ui,
    session: &mut MapEditorSession,
    edit_level: &mut CurrentEditLevel,
    view: &mut ViewMode,
    isolate: &mut IsolateView,
    #[cfg_attr(
        not(debug_assertions),
        expect(unused_variables, reason = "save-only in debug")
    )]
    save_name: &mut String,
    map: &crate::editor_map::EditorMap,
    registry: Option<&gdtf_battle_sim::terrain::def::TerrainDefRegistry>,
    themes: Option<&UuidThemeRegistry>,
    rail_state: &mut RailUiState,
) {
    ui.heading("Prefab");
    ui.separator();

    size_fields::size_fields(ui, session, edit_level);
    ui.separator();
    level_rail::level_rail(
        ui,
        &mut RailCtx {
            map,
            session,
            edit_level,
            registry,
            state: rail_state,
        },
    );
    ui.separator();
    view_toggle(ui, view);
    isolate_toggle(ui, isolate);

    #[cfg(debug_assertions)]
    {
        ui.separator();
        save_control(ui, session, map, registry, themes, save_name);
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = themes;
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

/// panic). Debug-only — the whole prefab save path is `#[cfg(debug_assertions)]`.
#[cfg(debug_assertions)]
fn save_control(
    ui: &mut egui::Ui,
    session: &MapEditorSession,
    map: &crate::editor_map::EditorMap,
    registry: Option<&gdtf_battle_sim::terrain::def::TerrainDefRegistry>,
    themes: Option<&UuidThemeRegistry>,
    save_name: &mut String,
) {
    ui.label("Prefab name");
    ui.text_edit_singleline(save_name);
    if !ui.button("Save prefab").clicked() {
        return;
    }
    let Some(registry) = registry else {
        bevy::log::error!("prefab save: terrain registry not resolved");
        return;
    };
    let theme_display = themes
        .and_then(|themes| themes.def(&session.theme()))
        .map_or_else(String::new, |def| (*def.display_name).clone());
    match crate::save::write_prefab(map, registry, session, &theme_display, save_name) {
        Ok(path) => bevy::log::info!("prefab save: wrote prefab to `{}`", path.display()),
        Err(err) => bevy::log::error!("prefab save: {err}"),
    }
}
