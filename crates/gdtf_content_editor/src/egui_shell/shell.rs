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
//! 3. left `Panel::left` — the palette / stats (TERRAIN's DEMOTED `.terrain_def.ron` preview —
//!    GTW-534 C2, THEME floor stats, PREFAB tile palette),
//! 4. right `Panel::right` — the active mode's form (THEME field stack, PREFAB controls; TERRAIN is
//!    idle here since GTW-534 C1 moved its controls to the central primary region),
//! 5. [`CentralPanel`](bevy_egui::egui::CentralPanel) — the primary region (TERRAIN's stat fields +
//!    sprite-grid picker as the primary focus — GTW-534 C1, THEME's terrain multi-select library
//!    with per-row sprite thumbnails as the primary focus — GTW-530 C1/C2, PREFAB the
//!    render-to-texture tile viewport — GTW-515 C4).
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
use gdtf_battle_sim::{
    level::{ThemeUuid, UuidThemeRegistry},
    terrain::def::TerrainDefRegistry,
};

use crate::{
    canvas::{CanvasZoom, CurrentEditLevel},
    editor_map::EditorMap,
    egui_shell::{
        prefab::{controls_ui, palette_ui, viewport_ui, viewport_ui::ViewportCtx},
        terrain_form_ui,
        theme_combo::{ThemeOption, theme_options},
        theme_form_ui,
    },
    hovered_cell::HoveredCell,
    mode::EditorMode,
    preview::{target::PreviewTarget, view::PreviewPan},
    session::MapEditorSession,
    terrain_form::TerrainDraft,
    theme_form::ThemeDraft,
    tile_atlas::TileAtlas,
};

/// The placeholder label shown when no theme is selected (the [`ThemeUuid::nil`] sentinel) — the
/// status line's nil-theme text and the `ComboBox`'s empty preview.
const NO_THEME: &str = "—";

/// `EguiPrimaryContextPass` (in `Editing`): the WHOLE editor shell — mode tabs + global theme
/// `ComboBox` (top), the status line (bottom), the per-mode palette/stats (left), the active mode's
/// form (right), and the per-mode viewport (central — the PREFAB render-to-texture viewport for
/// GTW-515 C4), declared outermost-first with the central panel last (C1.3 — the load-bearing egui
/// panel order).
///
/// Every editor resource is state-scoped (inserted `OnEnter(Editing)`, removed `OnExit(Editing)` —
/// bevy-traps #1), so the mode + session + the TERRAIN draft are taken as `Option<ResMut<…>>` and
/// the system no-ops until they exist; the theme registry + the presenter tile-role table are
/// likewise `Option<Res<…>>` (the empty-registry `ComboBox` then offers nothing; an unresolved
/// `TileRoles` leaves the graphic picker fully enabled). Returns a `Result` so a missing primary
/// egui context (`ctx_mut()?`) is handled, never unwrapped (the workspace lints deny
/// `unwrap`/`expect`).
#[expect(
    clippy::too_many_arguments,
    reason = "the whole-editor egui system draws ALL panels in one pass (the EguiPrimaryContextPass \
              requirement — bevy-traps #8); each param is a distinct Bevy SystemParam (the egui \
              context, the state-scoped mutable drafts + mode + session + the PREFAB model borrows \
              — map / edit-level / hovered / zoom / pan — and the read-only registries + tile atlas \
              + preview target); Bevy's injection model cannot reduce this without a wrapper \
              resource that changes the crate's API surface"
)]
#[expect(
    clippy::too_many_lines,
    reason = "egui panels CANNOT overlap and MUST be declared in one system, outermost-first with \
              the central panel last (bevy-traps #8) — the whole editor shell (top/bottom/left/ \
              right/central, each branching over the three Workbench modes) is one indivisible \
              EguiPrimaryContextPass system; the per-mode DRAW bodies are already factored into the \
              mode-specific `*_form_ui` / `prefab` modules, so what remains here is the irreducible \
              panel-declaration skeleton"
)]
pub(crate) fn editor_egui_ui(
    mut contexts: EguiContexts,
    mut prefab_save_name: Local<String>,
    mode: Option<ResMut<EditorMode>>,
    session: Option<ResMut<MapEditorSession>>,
    themes: Option<Res<UuidThemeRegistry>>,
    terrain_draft: Option<ResMut<TerrainDraft>>,
    theme_draft: Option<ResMut<ThemeDraft>>,
    terrain_registry: Option<Res<TerrainDefRegistry>>,
    roles: Option<Res<TileRoles>>,
    prefab: PrefabParams,
) -> Result {
    let (Some(mut mode), Some(mut session), Some(mut terrain_draft), Some(mut theme_draft)) =
        (mode, session, terrain_draft, theme_draft)
    else {
        return Ok(());
    };
    // Unpack the PREFAB model borrows (all state-scoped — bevy-traps #1). PREFAB mode no-ops until
    // they exist; TERRAIN / THEME modes never touch them, so a missing prefab resource does not
    // block those modes.
    let PrefabParams {
        mut map,
        mut edit_level,
        mut hovered,
        mut zoom,
        mut pan,
        atlas,
        preview_target,
    } = prefab;

    // Resolve the egui texture ids the PREFAB panels draw (the palette sprite sheet + the preview
    // render target) BEFORE borrowing `ctx_mut()` — `image_id` takes `&self`, so it must run before
    // the exclusive `ctx_mut()` borrow (bevy_egui 0.41). `None` until the atlas / target register.
    let sheet_id = atlas
        .as_deref()
        .and_then(|atlas| contexts.image_id(&atlas.image()));
    let preview_id = preview_target
        .as_deref()
        .and_then(|target| contexts.image_id(&target.image_handle()));

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

    // C3.2: when entering THEME mode with a theme already selected in the session, auto-load that
    // theme's def into the form so the author edits the live definition. This is checked every
    // frame; the `theme_form_ui::resolve_autoload` returns `None` for a nil theme / absent
    // registry, so it no-ops until a real theme resolves. The comparison avoids redundant
    // reinitialisation across frames by only loading when the form's current key differs from the
    // session theme (a new selection or a first-enter with a pre-selected theme).
    if *mode == EditorMode::Theme
        && let Some(themes_res) = themes.as_deref()
        && let Some(def) = theme_form_ui::resolve_autoload(session.theme(), themes_res)
        && theme_draft.key() != def.key
    {
        theme_form_ui::load_theme_into_form(&mut theme_draft, def);
    }

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

    // 3. LEFT — the palette / stats region. In TERRAIN mode (GTW-534 C2) it now hosts the DEMOTED
    //    `.terrain_def.ron` live preview — relocated OFF the central region to this secondary side
    //    strip: still present + live-updating, but no longer dominating (the stat fields + sprite
    //    picker take the central primary space instead). In THEME mode (C3) it shows the resolved
    //    floor-terrain stats readout; PREFAB keeps the tile palette.
    egui::Panel::left("editor_palette").show(&mut viewport_ui, |ui| match *mode {
        EditorMode::Terrain => {
            terrain_form_ui::ron_preview(ui, &terrain_draft);
        }
        EditorMode::Theme => {
            theme_form_ui::stats_panel(ui, &theme_draft, terrain_registry.as_deref());
        }
        EditorMode::Prefab => {
            palette_ui::palette_panel(
                ui,
                &mut session,
                themes.as_deref(),
                terrain_registry.as_deref(),
                roles.as_deref(),
                atlas.as_deref(),
                sheet_id,
            );
        }
    });

    // 4. RIGHT — the ACTIVE mode's form (an in-UI branch). TERRAIN no longer renders here (GTW-534
    //    C1 folded its picker + field stack into the CENTRAL primary region below); THEME is the
    //    real form (C3 / GTW-514); PREFAB is the real controls (C4 / GTW-515) — the grid-size
    //    fields, the level nav, and the debug Save.
    egui::Panel::right("editor_mode_form").show(&mut viewport_ui, |ui| match *mode {
        // TERRAIN's controls are central now (GTW-534 C1); the right panel is intentionally idle in
        // TERRAIN mode so nothing competes with the central stats + sprite picker.
        EditorMode::Terrain => {}
        EditorMode::Theme => {
            theme_form_ui::field_stack(ui, &mut theme_draft, terrain_registry.as_deref());
        }
        EditorMode::Prefab => {
            if let (Some(edit_level), Some(map)) = (edit_level.as_deref_mut(), map.as_deref()) {
                controls_ui::controls_panel(
                    ui,
                    &mut session,
                    edit_level,
                    &mut prefab_save_name,
                    map,
                    terrain_registry.as_deref(),
                    themes.as_deref(),
                );
            }
        }
    });

    // 5. CENTRAL — the viewport / primary region (LAST: egui fills the residual space with it). In
    //    TERRAIN mode (GTW-534 C1) it is now the PRIMARY focus: the sprite-grid graphic picker
    //    (GTW-516) + the terrain stat field stack, side by side — the two things authoring a terrain
    //    is about (the demoted `.terrain_def.ron` preview lives in the LEFT secondary strip). In
    //    THEME mode (GTW-530 C1/C2) it is now the PRIMARY focus: the terrain multi-select library
    //    with per-row `[sprite] name [Kind]` thumbnails (the `.terrain_theme.ron` preview was
    //    REMOVED — C1). In PREFAB mode (C4 / GTW-515) it shows the render-to-texture viewport
    //    (click-to-paint + hover ghost + wheel-zoom + right-drag pan).
    egui::CentralPanel::default().show(&mut viewport_ui, |ui| match *mode {
        EditorMode::Terrain => {
            terrain_form_ui::primary_panel(
                ui,
                &mut terrain_draft,
                &session,
                themes.as_deref(),
                roles.as_deref(),
                sheet_id,
            );
        }
        EditorMode::Theme => {
            theme_form_ui::terrain_library_panel(
                ui,
                &mut theme_draft,
                terrain_registry.as_deref(),
                roles.as_deref(),
                sheet_id,
            );
        }
        EditorMode::Prefab => {
            if let (Some(map), Some(edit_level), Some(hovered), Some(zoom), Some(pan)) = (
                map.as_deref_mut(),
                edit_level.as_deref(),
                hovered.as_deref_mut(),
                zoom.as_deref_mut(),
                pan.as_deref_mut(),
            ) {
                let mut vp = ViewportCtx {
                    map,
                    session: &session,
                    edit_level,
                    hovered,
                    zoom,
                    pan,
                    registry: terrain_registry.as_deref(),
                    themes: themes.as_deref(),
                    roles: roles.as_deref(),
                };
                viewport_ui::viewport_panel(ui, &mut vp, preview_id);
            } else {
                ui.heading("Viewport");
                ui.label("Preparing prefab preview…");
            }
        }
    });

    Ok(())
}

/// The state-scoped PREFAB-mode model borrows the shell threads into the PREFAB panels (GTW-515) —
/// grouped into one `#[derive(SystemParam)]` bundle so the shell system's argument list stays
/// legible (the borrows are distinct `SystemParam`s; bundling them is the standard Bevy pattern for a
/// system that would otherwise take too many). Every field is `Option` because each resource is
/// state-scoped (inserted `OnEnter(Editing)`, removed `OnExit(Editing)` — bevy-traps #1), so
/// TERRAIN / THEME modes (which never touch them) tolerate their absence and PREFAB mode no-ops
/// until they exist.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct PrefabParams<'w> {
    /// The paintable map model (mutated by click-to-paint).
    map:            Option<ResMut<'w, EditorMap>>,
    /// The current edit storey (stepped by the level nav; read by the viewport paint).
    edit_level:     Option<ResMut<'w, CurrentEditLevel>>,
    /// The hovered-cell model (written each frame from the viewport pointer).
    hovered:        Option<ResMut<'w, HoveredCell>>,
    /// The owned zoom target (folded from the viewport wheel — set-to-target).
    zoom:           Option<ResMut<'w, CanvasZoom>>,
    /// The owned pan target (folded from the viewport right-drag — set-to-target).
    pan:            Option<ResMut<'w, PreviewPan>>,
    /// The terrain tile atlas (the palette sprite thumbnails draw over it).
    atlas:          Option<Res<'w, TileAtlas>>,
    /// The offscreen preview render target (the viewport draws its egui-registered image).
    preview_target: Option<Res<'w, PreviewTarget>>,
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
