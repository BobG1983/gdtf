//! The editor's single egui UI system + its panel layout (GTW-512 C1).
//!
//! [`editor_egui_ui`] runs in the [`EguiPrimaryContextPass`](bevy_egui::EguiPrimaryContextPass)
//! schedule (NOT `Update` — bevy-traps: a `Update` system calling `ctx_mut()` fights the egui
//! begin/end-pass plumbing), gated `run_if(in_state(EditorState::Editing))`. It declares the egui
//! panels in the LOAD-BEARING order egui requires — outermost-first, the
//! [`CentralPanel`](bevy_egui::egui::CentralPanel) LAST (egui panels cannot overlap; a wrong order
//! steals the central region's space). egui 0.35 unified the side / top / bottom panels into one
//! [`Panel`](bevy_egui::egui::Panel) shown into a root `Ui` (`Panel::top` / `::bottom` / `::left` /
//! `::right`):
//!
//! 1. top `Panel::top` — mode tabs + global theme `ComboBox`,
//! 2. bottom `Panel::bottom` — the status line,
//! 3. left `Panel::left` — the palette / stats placeholder,
//! 4. right `Panel::right` — the active mode's form,
//! 5. [`CentralPanel`](bevy_egui::egui::CentralPanel) — the viewport placeholder.
//!
//! The mode switch is an IN-UI branch inside the right panel (`if mode == Prefab {…} else if …`)
//! — there is no `bevy_ui` `mode_host` / `Visibility`-container machinery any more (C1.3 deleted
//! it). The mode tabs are egui [`selectable_value`](bevy_egui::egui::Ui::selectable_value)s over
//! the kept [`EditorMode`](crate::mode::EditorMode) resource; the `1`/`2`/`3` hotkeys are still
//! handled by [`mode_hotkeys`](crate::mode::mode_hotkeys) in `Update` (UI-agnostic, kept). The
//! theme `ComboBox` folds a selection into the [`MapEditorSession`] exactly as the old
//! `apply_theme_selection` did (it resolves the chosen theme's default-floor from the registry and
//! calls [`MapEditorSession::select_theme`]).

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use gdtf_battle_presenter::TileRoles;
use gdtf_battle_sim::level::{ThemeUuid, UuidThemeRegistry};

use crate::{
    egui_shell::{
        forms, terrain_form_ui,
        theme_combo::{ThemeOption, theme_options},
    },
    mode::EditorMode,
    session::MapEditorSession,
    terrain_form::TerrainDraft,
};

/// The placeholder label shown when no theme is selected (the [`ThemeUuid::nil`] sentinel) — the
/// status line's nil-theme text and the `ComboBox`'s empty preview.
const NO_THEME: &str = "—";

/// `EguiPrimaryContextPass` (in `Editing`): the WHOLE editor shell — mode tabs + global theme
/// `ComboBox` (top), the status line (bottom), the palette/stats placeholder (left), the active
/// mode's form (right), and the viewport placeholder (central), declared outermost-first with the
/// central panel last (C1.3 — the load-bearing egui panel order).
///
/// Every editor resource is state-scoped (inserted `OnEnter(Editing)`, removed `OnExit(Editing)` —
/// bevy-traps #1), so the mode + session + the TERRAIN draft are taken as `Option<ResMut<…>>` and
/// the system no-ops until they exist; the theme registry + the presenter tile-role table are
/// likewise `Option<Res<…>>` (the empty-registry `ComboBox` then offers nothing; an unresolved
/// `TileRoles` leaves the graphic picker fully enabled). Returns a `Result` so a missing primary
/// egui context (`ctx_mut()?`) is handled, never unwrapped (the workspace lints deny
/// `unwrap`/`expect`).
pub(crate) fn editor_egui_ui(
    mut contexts: EguiContexts,
    mode: Option<ResMut<EditorMode>>,
    session: Option<ResMut<MapEditorSession>>,
    themes: Option<Res<UuidThemeRegistry>>,
    terrain_draft: Option<ResMut<TerrainDraft>>,
    roles: Option<Res<TileRoles>>,
) -> Result {
    let (Some(mut mode), Some(mut session), Some(mut terrain_draft)) =
        (mode, session, terrain_draft)
    else {
        return Ok(());
    };
    let ctx = contexts.ctx_mut()?;
    // egui 0.35 / bevy_egui 0.41 show panels INTO a root `Ui` (the panel `show` takes `&mut Ui`,
    // NOT a `&Context` — bevy-traps #8). Build the background-layer viewport `Ui` over the whole
    // context rect (the bevy_egui `ui.rs` example idiom), then declare the panels into it.
    let mut viewport_ui = egui::Ui::new(
        ctx.clone(),
        "editor_viewport".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );
    let options = theme_options(themes.as_deref());

    // 1. TOP — mode tabs (left) + the global theme `ComboBox` (right). Full-width bars are declared
    //    FIRST so they span edge-to-edge; the side panels then fit between them.
    egui::Panel::top("editor_top_bar").show(&mut viewport_ui, |ui| {
        ui.horizontal(|ui| {
            mode_tabs(ui, &mut mode);
            ui.separator();
            theme_combo_box(ui, &options, themes.as_deref(), &mut session);
        });
    });

    // 2. BOTTOM — the status line.
    egui::Panel::bottom("editor_status_bar").show(&mut viewport_ui, |ui| {
        ui.label(status_line(*mode, &session, themes.as_deref()));
    });

    // 3. LEFT — the palette / stats region. In TERRAIN mode (C2) it hosts the graphic-role picker
    //    (the 10 `TileRoles` keys, active highlighted); other modes keep the palette placeholder.
    egui::Panel::left("editor_palette").show(&mut viewport_ui, |ui| {
        if *mode == EditorMode::Terrain {
            terrain_form_ui::graphic_picker(ui, &mut terrain_draft, roles.as_deref());
        } else {
            ui.heading("Palette");
            ui.separator();
            ui.label("Tile palette + selected-tile stats.");
        }
    });

    // 4. RIGHT — the ACTIVE mode's form (an in-UI branch). TERRAIN is the real form (C2 / GTW-513);
    //    THEME / PREFAB are still stubbed (C3 / C4).
    egui::Panel::right("editor_mode_form").show(&mut viewport_ui, |ui| match *mode {
        EditorMode::Terrain => {
            terrain_form_ui::field_stack(ui, &mut terrain_draft, &session, themes.as_deref());
        }
        EditorMode::Theme => forms::theme_form(ui),
        EditorMode::Prefab => forms::prefab_form(ui),
    });

    // 5. CENTRAL — the viewport / preview (LAST: egui fills the residual space with it). In TERRAIN
    //    mode (C2) it shows the live `.terrain_def.ron` preview; other modes keep the viewport
    //    placeholder (the texture viewport is C4 / GTW-515).
    egui::CentralPanel::default().show(&mut viewport_ui, |ui| {
        if *mode == EditorMode::Terrain {
            terrain_form_ui::ron_preview(ui, &terrain_draft);
        } else {
            ui.heading("Viewport");
            ui.label("Canvas / texture viewport (C4 / GTW-515).");
        }
    });

    Ok(())
}

/// Draw the `[TERRAIN | THEME | PREFAB]` mode tabs as egui
/// [`selectable_value`](egui::Ui::selectable_value)s over the [`EditorMode`] resource — a click
/// sets the mode in place (the `1`/`2`/`3` hotkeys do the same via
/// [`mode_hotkeys`](crate::mode::mode_hotkeys)). The tab ORDER is [`EditorMode::TAB_ORDER`] and
/// each label is [`EditorMode::tab_label`], so the egui tabs match the old shell's tabs.
fn mode_tabs(ui: &mut egui::Ui, mode: &mut EditorMode) {
    for option in EditorMode::TAB_ORDER {
        ui.selectable_value(mode, option, option.tab_label());
    }
}

/// Draw the global theme [`ComboBox`](egui::ComboBox) — its options are the sorted
/// [`theme_options`], the currently-selected theme's display name is the preview, and choosing a
/// row folds the selection into the session exactly as the old `apply_theme_selection` did
/// (resolving the chosen theme's default-floor from the registry, then calling
/// [`MapEditorSession::select_theme`]).
fn theme_combo_box(
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
fn status_line(
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
