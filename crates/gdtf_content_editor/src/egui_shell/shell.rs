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
//! 4. right `Panel::right` — the active mode's form
//!    ([`mode_panels::right_panel`] — the per-mode dispatch),
//! 5. [`CentralPanel`](bevy_egui::egui::CentralPanel) — the primary region
//!    ([`mode_panels::central_panel`] — the per-mode dispatch).
//!
//! The two PER-MODE dispatches (panels 4 + 5) live in the
//! [`mode_panels`](super::mode_panels) sibling since GTW-670 (module-layout bands: they
//! grow one arm per Workbench mode; the shell only changes when the panel LAYOUT does —
//! the `autoload.rs` / `textures.rs` seam continuation). The mode tabs are egui
//! [`selectable_value`](bevy_egui::egui::Ui::selectable_value)s over the kept
//! [`EditorMode`](crate::mode::EditorMode) resource; the `1`–`9` hotkeys are still handled by
//! [`mode_hotkeys`](crate::mode::mode_hotkeys) in `Update` (UI-agnostic, kept). The theme
//! `ComboBox` folds a selection into the [`MapEditorSession`] exactly as the old
//! `apply_theme_selection` did.
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
        autoload::{ModeSyncBundles, run_form_syncs},
        chrome::{mode_tabs, status_line, theme_combo_box},
        mode_panels::{ModePanelsCtx, central_panel, right_panel},
        params::{
            ArmorParams, AttachmentParams, GangParams, InjuryParams, PrefabParams, SpriteParams,
            WeaponParams,
        },
        prefab::palette_ui,
        terrain_form_ui,
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
/// form (right — the [`mode_panels`](super::mode_panels) dispatch), and the per-mode viewport
/// (central — same dispatch, declared LAST per C1.3, the load-bearing egui panel order).
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
              bundles — prefab / gang / armor / injury / sprite / attachment / weapon — and the \
              read-only registries); Bevy's injection model cannot reduce this without a wrapper \
              resource that changes the crate's API surface"
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
    mut attachment_mode: AttachmentParams,
    mut weapon_mode: WeaponParams,
) -> Result {
    let (Some(mut mode), Some(mut session), Some(mut terrain_draft), Some(mut theme_draft)) =
        (mode, session, terrain_draft, theme_draft)
    else {
        return Ok(());
    };

    // The PRE-PANEL per-mode model-sync / autoload FAN-OUT (split into
    // `egui_shell::autoload` at the GTW-479-flagged seam — GTW-654; the whole roster
    // call moved there in GTW-669): each runner self-gates on its mode + borrows and
    // is multipass-idempotent (bevy-traps #8). Runs BEFORE the texture-id resolution
    // below so the sprite autoload's seeded source is what the preview resolver loads
    // this same frame (GTW-664).
    run_form_syncs(
        *mode,
        &session,
        themes.as_deref(),
        &mut theme_draft,
        ModeSyncBundles {
            gang:       &mut gang,
            armor:      &mut armor_mode,
            injury:     &mut injury_mode,
            sprite:     &mut sprite_mode,
            attachment: &mut attachment_mode,
            weapon:     &mut weapon_mode,
        },
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
        // GANG / ARMOR / INJURY / SPRITE / ATTACHMENT / WEAPON modes keep this
        // secondary strip intentionally idle (GTW-636 / GTW-479 / GTW-654 / GTW-664 /
        // GTW-669 / GTW-670): the member list / piece grid / def editors are the
        // central primary focus and the form controls live in the right panel, so
        // nothing competes here (the TERRAIN right-panel precedent).
        EditorMode::Gang
        | EditorMode::Armor
        | EditorMode::Injury
        | EditorMode::Sprite
        | EditorMode::Attachment
        | EditorMode::Weapon => {}
    });

    // 4 + 5. RIGHT (the active mode's form) then CENTRAL (the primary region, LAST —
    // egui fills the residual space with it): the per-mode dispatches, moved to
    // `egui_shell::mode_panels` at the GTW-670 band seam. One borrow context threads
    // every per-mode model into both.
    let mut panel_ctx = ModePanelsCtx {
        session:          &mut session,
        terrain_draft:    &mut terrain_draft,
        theme_draft:      &mut theme_draft,
        prefab_save_name: &mut prefab_save_name,
        themes:           themes.as_deref(),
        terrain_registry: terrain_registry.as_deref(),
        weapons:          weapons.as_deref(),
        textures:         &textures,
        prefab:           &mut prefab,
        gang:             &mut gang,
        armor:            &mut armor_mode,
        injury:           &mut injury_mode,
        sprite:           &mut sprite_mode,
        attachment:       &mut attachment_mode,
        weapon:           &mut weapon_mode,
    };
    right_panel(&mut viewport_ui, *mode, &mut panel_ctx);
    central_panel(&mut viewport_ui, *mode, &mut panel_ctx);

    Ok(())
}
