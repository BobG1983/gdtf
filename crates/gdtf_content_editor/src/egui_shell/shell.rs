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
//!
//! Since GTW-664 the per-mode MODEL borrows stay inside their `params` bundles (direct field
//! access, no unpack block) — the bundles grow when a mode's model surface does, the shell only
//! when the PANEL layout does.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use gdtf_battle_sim::{
    level::UuidThemeRegistry, terrain::def::TerrainDefRegistry, weapon::WeaponRegistry,
};

use crate::{
    egui_shell::{
        armor_form_ui,
        autoload::{
            armor_form_sync, gang_form_sync, injury_form_sync, sprite_form_sync, theme_form_sync,
        },
        chrome::{mode_tabs, status_line, theme_combo_box},
        gang_form_ui, injury_form_ui,
        params::{ArmorParams, GangParams, InjuryParams, PrefabParams, SpriteParams},
        prefab::{controls_ui, palette_ui, viewport_ui, viewport_ui::ViewportCtx},
        sprite_form_ui, terrain_form_ui,
        textures::resolve_panel_textures,
        theme_combo::theme_options,
        theme_form_ui,
    },
    mode::EditorMode,
    session::MapEditorSession,
    terrain_form::TerrainDraft,
    theme_form::ThemeDraft,
};

/// `EguiPrimaryContextPass` (in `Editing`): the WHOLE editor shell — mode tabs + global theme
/// `ComboBox` (top), the status line (bottom), the per-mode palette/stats (left), the active mode's
/// form (right), and the per-mode viewport (central — the PREFAB render-to-texture viewport for
/// GTW-515 C4), declared outermost-first with the central panel last (C1.3 — the load-bearing egui
/// panel order).
///
/// Every editor resource is state-scoped (inserted `OnEnter(Editing)`, removed `OnExit(Editing)` —
/// bevy-traps #1), so the mode + session + the TERRAIN draft are taken as `Option<ResMut<…>>` and
/// the system no-ops until they exist; the theme registry + the
/// weapon registry (the GTW-574 Emplacement mounted-weapon combo's option source) are likewise
/// `Option<Res<…>>` (the empty-registry `ComboBox` then offers nothing). The sprite thumbnails
/// resolve through the GTW-663 `SpriteDefRegistry` the SPRITE bundle already carries (GTW-665 —
/// the ONE def-driven resolution the battle renderer uses; an unresolved registry leaves every
/// thumb on its fallback). Returns a `Result` so a missing primary
/// egui context (`ctx_mut()?`) is handled, never unwrapped (the workspace lints deny
/// `unwrap`/`expect`).
#[expect(
    clippy::too_many_arguments,
    reason = "the whole-editor egui system draws ALL panels in one pass (the EguiPrimaryContextPass \
              requirement — bevy-traps #8); each param is a distinct Bevy SystemParam (the egui \
              context, the state-scoped mutable drafts + mode + session, the per-mode model \
              bundles — prefab / gang / armor / injury / sprite — and the read-only registries); \
              Bevy's injection model cannot reduce this without a wrapper resource that changes \
              the crate's API surface"
)]
#[expect(
    clippy::too_many_lines,
    reason = "egui panels CANNOT overlap and MUST be declared in one system, outermost-first with \
              the central panel last (bevy-traps #8) — the whole editor shell (top/bottom/left/ \
              right/central, each branching over the Workbench modes) is one indivisible \
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
    weapons: Option<Res<WeaponRegistry>>,
    mut prefab: PrefabParams,
    mut gang: GangParams,
    mut armor_mode: ArmorParams,
    mut injury_mode: InjuryParams,
    mut sprite_mode: SpriteParams,
) -> Result {
    let (Some(mut mode), Some(mut session), Some(mut terrain_draft), Some(mut theme_draft)) =
        (mode, session, terrain_draft, theme_draft)
    else {
        return Ok(());
    };

    // The PRE-PANEL per-mode model-sync / autoload block (split into
    // `egui_shell::autoload` at the GTW-479-flagged seam — GTW-654): each runner
    // self-gates on its mode + borrows and is multipass-idempotent (bevy-traps #8).
    // Runs BEFORE the texture-id resolution below so the sprite autoload's seeded
    // source is what the preview resolver loads this same frame (GTW-664).
    theme_form_sync(*mode, &session, themes.as_deref(), &mut theme_draft);
    gang_form_sync(*mode, gang.draft.as_deref_mut(), gang.gangs.as_deref());
    armor_form_sync(
        *mode,
        armor_mode.draft.as_deref_mut(),
        armor_mode.registry.as_deref(),
    );
    injury_form_sync(
        *mode,
        injury_mode.draft.as_deref_mut(),
        injury_mode.registry.as_deref(),
        injury_mode.weighting.as_deref_mut(),
        injury_mode.tables.as_deref(),
    );
    sprite_form_sync(
        *mode,
        sprite_mode.draft.as_deref_mut(),
        sprite_mode.registry.as_deref(),
    );

    // Resolve the egui texture ids the panels draw (the palette sprite sheet, the prefab
    // preview render target, and the GTW-664 sprite-source preview) BEFORE borrowing
    // `ctx_mut()` — split into `egui_shell::textures` at the GTW-664 natural seam.
    let textures = resolve_panel_textures(&mut contexts, *mode, &prefab, &mut sprite_mode);

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
                sprite_mode.registry.as_deref(),
                &textures.sprites,
            );
        }
        // GANG / ARMOR / INJURY / SPRITE modes keep this secondary strip intentionally
        // idle (GTW-636 / GTW-479 / GTW-654 / GTW-664): the member list / piece grid /
        // def editors are the central primary focus and the form controls live in the
        // right panel, so nothing competes here (the TERRAIN right-panel precedent).
        EditorMode::Gang | EditorMode::Armor | EditorMode::Injury | EditorMode::Sprite => {}
    });

    // 4. RIGHT — the ACTIVE mode's form (an in-UI branch). TERRAIN no longer renders here (GTW-534
    //    C1 folded its picker + field stack into the CENTRAL primary region below); THEME is the
    //    real form (C3 / GTW-514); PREFAB is the real controls (C4 / GTW-515) — the grid-size
    //    fields, the GTW-595 level rail, and the debug Save.
    egui::Panel::right("editor_mode_form").show(&mut viewport_ui, |ui| match *mode {
        // TERRAIN's controls are central now (GTW-534 C1); the right panel is intentionally idle in
        // TERRAIN mode so nothing competes with the central stats + sprite picker.
        EditorMode::Terrain => {}
        EditorMode::Theme => {
            theme_form_ui::field_stack(ui, &mut theme_draft, terrain_registry.as_deref());
        }
        EditorMode::Prefab => {
            if let (Some(edit_level), Some(view), Some(isolate), Some(map)) = (
                prefab.edit_level.as_deref_mut(),
                prefab.view.as_deref_mut(),
                prefab.isolate.as_deref_mut(),
                prefab.map.as_deref(),
            ) {
                controls_ui::controls_panel(
                    ui,
                    &mut session,
                    edit_level,
                    view,
                    isolate,
                    &mut prefab_save_name,
                    map,
                    terrain_registry.as_deref(),
                    themes.as_deref(),
                    &mut prefab.rail_state,
                );
            }
        }
        EditorMode::Gang => {
            if let Some(draft) = gang.draft.as_deref_mut() {
                gang_form_ui::field_stack(ui, draft, gang.gangs.as_deref());
            }
        }
        EditorMode::Armor => {
            if let Some(draft) = armor_mode.draft.as_deref_mut() {
                armor_form_ui::field_stack(ui, draft, armor_mode.registry.as_deref());
            }
        }
        EditorMode::Injury => {
            if let Some(draft) = injury_mode.draft.as_deref_mut() {
                injury_form_ui::field_stack(ui, draft, injury_mode.registry.as_deref());
            }
        }
        // GTW-664: load / name / New sprite / debug Save — the Gang/Armor field-stack
        // parity over the sprite draft + the GTW-663 registry.
        EditorMode::Sprite => {
            if let Some(draft) = sprite_mode.draft.as_deref_mut() {
                sprite_form_ui::field_stack(ui, draft, sprite_mode.registry.as_deref());
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
                sprite_mode.registry.as_deref(),
                weapons.as_deref(),
                &textures.sprites,
            );
        }
        EditorMode::Theme => {
            theme_form_ui::terrain_library_panel(
                ui,
                &mut theme_draft,
                terrain_registry.as_deref(),
                sprite_mode.registry.as_deref(),
                &textures.sprites,
            );
        }
        EditorMode::Prefab => {
            if let (Some(map), Some(edit_level), Some(hovered), Some(zoom), Some(pan)) = (
                prefab.map.as_deref_mut(),
                prefab.edit_level.as_deref(),
                prefab.hovered.as_deref_mut(),
                prefab.zoom.as_deref_mut(),
                prefab.pan.as_deref_mut(),
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
                };
                viewport_ui::viewport_panel(ui, &mut vp, textures.preview_id);
            } else {
                ui.heading("Viewport");
                ui.label("Preparing prefab preview…");
            }
        }
        // GTW-636: the member-list editor is the GANG mode's PRIMARY focus — one
        // collapsible per-member editor (name / loadout dropdowns / attributes /
        // derived stats / remove) over the draft's sim records.
        EditorMode::Gang => {
            if let Some(draft) = gang.draft.as_deref_mut() {
                gang_form_ui::members_panel(
                    ui,
                    draft,
                    weapons.as_deref(),
                    gang.melee.as_deref(),
                    gang.armor.as_deref(),
                    gang.tuning.as_deref(),
                );
            }
        }
        // GTW-479: the per-body-part piece grid is the ARMOR mode's PRIMARY focus —
        // one row per BodyPart (the four clamped stat drags + the ArmorType combo)
        // over the draft's sim record.
        EditorMode::Armor => {
            if let Some(draft) = armor_mode.draft.as_deref_mut() {
                armor_form_ui::pieces_panel(ui, draft);
            }
        }
        // GTW-654: the def editor (fields + the closed-palette effects list) and the
        // weighting section (C2) are the INJURY mode's PRIMARY focus, stacked in one
        // scroll area over the two drafts' sim records.
        EditorMode::Injury => {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if let Some(draft) = injury_mode.draft.as_deref_mut() {
                        injury_form_ui::def_panel(ui, draft);
                    }
                    if let Some(weighting) = injury_mode.weighting.as_deref_mut() {
                        ui.separator();
                        injury_form_ui::weighting_panel(
                            ui,
                            weighting,
                            injury_mode.registry.as_deref(),
                            injury_mode.tables.as_deref(),
                        );
                    }
                });
        }
        // GTW-664: the full def editor is the SPRITE mode's PRIMARY focus — source
        // picker, the visual anchor affordance (crosshair over the shell-resolved
        // preview), facings overrides, and animation rows, stacked in one scroll area.
        EditorMode::Sprite => {
            if let Some(draft) = sprite_mode.draft.as_deref_mut() {
                sprite_form_ui::primary_panel(
                    ui,
                    draft,
                    &mut sprite_mode.preview_cache,
                    textures.sprite_preview.as_ref(),
                );
            }
        }
    });

    Ok(())
}
