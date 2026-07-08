//! The CENTRAL primary-panel dispatch — one arm per Workbench mode (moved out of
//! `shell.rs` at the GTW-670 band seam alongside [`right`](super::right)). The central
//! panel is declared LAST by the shell's order contract: egui fills the residual space
//! with it, so this dispatch must run after every other panel is declared.

use bevy_egui::egui;

use super::ctx::ModePanelsCtx;
use crate::{
    egui_shell::{
        armor_form_ui, attachment_form_ui, gang_form_ui, injury_form_ui,
        prefab::{viewport_ui, viewport_ui::ViewportCtx},
        sprite_form_ui, terrain_form_ui, theme_form_ui, weapon_form_ui,
    },
    mode::EditorMode,
};

/// Declare the CENTRAL panel — the viewport / primary region (the panel-5 slot, LAST).
/// In TERRAIN mode (GTW-534 C1) it is the PRIMARY focus: the sprite-grid graphic picker
/// (GTW-516) + the terrain stat field stack. In THEME mode (GTW-530 C1/C2) the terrain
/// multi-select library with per-row sprite thumbnails. In PREFAB mode (GTW-515 C4) the
/// render-to-texture viewport (click-to-paint + hover ghost + wheel-zoom + right-drag
/// pan). The GANG / ARMOR / INJURY / SPRITE / ATTACHMENT / WEAPON modes host their full
/// def editors here (each mode's `*_form_ui` primary panel).
pub(in crate::egui_shell) fn central_panel(
    viewport_ui: &mut egui::Ui,
    mode: EditorMode,
    ctx: &mut ModePanelsCtx<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
) {
    egui::CentralPanel::default().show(viewport_ui, |ui| match mode {
        EditorMode::Terrain => {
            terrain_form_ui::primary_panel(
                ui,
                ctx.terrain_draft,
                ctx.session,
                ctx.themes,
                ctx.sprite.registry.as_deref(),
                ctx.weapons,
                &ctx.textures.sprites,
            );
        }
        EditorMode::Theme => {
            theme_form_ui::terrain_library_panel(
                ui,
                ctx.theme_draft,
                ctx.terrain_registry,
                ctx.sprite.registry.as_deref(),
                &ctx.textures.sprites,
            );
        }
        EditorMode::Prefab => prefab_viewport(ui, ctx),
        // GTW-636: the member-list editor is the GANG mode's PRIMARY focus — one
        // collapsible per-member editor (name / loadout dropdowns / attributes /
        // derived stats / remove) over the draft's sim records.
        EditorMode::Gang => {
            if let Some(draft) = ctx.gang.draft.as_deref_mut() {
                gang_form_ui::members_panel(
                    ui,
                    draft,
                    ctx.weapons,
                    ctx.gang.melee.as_deref(),
                    ctx.gang.armor.as_deref(),
                    ctx.gang.tuning.as_deref(),
                );
            }
        }
        // GTW-479: the per-body-part piece grid is the ARMOR mode's PRIMARY focus —
        // one row per BodyPart (the four clamped stat drags + the ArmorType combo)
        // over the draft's sim record.
        EditorMode::Armor => {
            if let Some(draft) = ctx.armor.draft.as_deref_mut() {
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
                    if let Some(draft) = ctx.injury.draft.as_deref_mut() {
                        injury_form_ui::def_panel(ui, draft);
                    }
                    if let Some(weighting) = ctx.injury.weighting.as_deref_mut() {
                        ui.separator();
                        injury_form_ui::weighting_panel(
                            ui,
                            weighting,
                            ctx.injury.registry.as_deref(),
                            ctx.injury.tables.as_deref(),
                        );
                    }
                });
        }
        // GTW-664: the full def editor is the SPRITE mode's PRIMARY focus — source
        // picker, the visual anchor affordance (crosshair over the shell-resolved
        // preview), facings overrides, and animation rows, stacked in one scroll area.
        EditorMode::Sprite => {
            if let Some(draft) = ctx.sprite.draft.as_deref_mut() {
                sprite_form_ui::primary_panel(
                    ui,
                    draft,
                    &mut ctx.sprite.preview_cache,
                    ctx.textures.sprite_preview.as_ref(),
                );
            }
        }
        // GTW-669: the item editor (display name / slot / the closed 13-effect list)
        // is the ATTACHMENT mode's PRIMARY focus, in one scroll area over the draft's
        // sim record (the injury def-panel shape).
        EditorMode::Attachment => {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if let Some(draft) = ctx.attachment.draft.as_deref_mut() {
                        attachment_form_ui::def_panel(ui, draft);
                    }
                });
        }
        // GTW-670: the full 18-field WeaponSpec editor is the WEAPON mode's PRIMARY
        // focus — the collapsible-section stack (stats / handling / magazine / fire
        // modes / slots / attachments / dot / on-death) in one scroll area over the
        // draft's sim record (the injury / attachment scroll-stack shape), with the
        // GTW-619 attachment registry as the key combos' option source.
        EditorMode::Weapon => {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if let Some(draft) = ctx.weapon.draft.as_deref_mut() {
                        weapon_form_ui::def_panel(ui, draft, ctx.weapon.attachments.as_deref());
                    }
                });
        }
    });
}

/// The PREFAB central arm — the render-to-texture viewport (GTW-515 C4:
/// click-to-paint + hover ghost + wheel-zoom + right-drag pan), or the placeholder
/// while the preview machinery is still preparing. Split out of [`central_panel`]
/// purely for the `too_many_lines` band as the WEAPON arm joined (GTW-670).
fn prefab_viewport(
    ui: &mut egui::Ui,
    ctx: &mut ModePanelsCtx<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
) {
    if let (Some(map), Some(edit_level), Some(hovered), Some(zoom), Some(pan)) = (
        ctx.prefab.map.as_deref_mut(),
        ctx.prefab.edit_level.as_deref(),
        ctx.prefab.hovered.as_deref_mut(),
        ctx.prefab.zoom.as_deref_mut(),
        ctx.prefab.pan.as_deref_mut(),
    ) {
        let mut vp = ViewportCtx {
            map,
            session: ctx.session,
            edit_level,
            hovered,
            zoom,
            pan,
            registry: ctx.terrain_registry,
            themes: ctx.themes,
        };
        viewport_ui::viewport_panel(ui, &mut vp, ctx.textures.preview_id);
    } else {
        ui.heading("Viewport");
        ui.label("Preparing prefab preview…");
    }
}
