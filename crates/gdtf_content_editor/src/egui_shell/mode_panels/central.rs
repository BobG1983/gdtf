use bevy_egui::egui;

use super::ctx::ModePanelsCtx;
use crate::{
    egui_shell::{
        armor_form_ui, attachment_form_ui, gang_form_ui, injury_form_ui, melee_weapon_form_ui,
        prefab::{viewport_ui, viewport_ui::ViewportCtx},
        sprite_form_ui,
        terrain_form_ui::{self, TerrainSaveContext},
        theme_form_ui, weapon_form_ui,
    },
    mode::EditorMode,
};

pub(in crate::egui_shell) fn central_panel(
    viewport_ui: &mut egui::Ui,
    mode: EditorMode,
    ctx: &mut ModePanelsCtx<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
) {
    egui::CentralPanel::default().show(viewport_ui, |ui| match mode {
        EditorMode::Terrain => {
            terrain_form_ui::primary_panel(
                ui,
                ctx.terrain_draft,
                TerrainSaveContext {
                    session:   ctx.session,
                    themes:    ctx.themes,
                    last_save: ctx.last_save,
                },
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
        EditorMode::Armor => {
            if let Some(draft) = ctx.armor.draft.as_deref_mut() {
                armor_form_ui::pieces_panel(ui, draft);
            }
        }
        EditorMode::Injury => injury_stack(ui, ctx),
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
        EditorMode::Attachment => {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if let Some(draft) = ctx.attachment.draft.as_deref_mut() {
                        attachment_form_ui::def_panel(ui, draft);
                    }
                });
        }
        EditorMode::Weapon => {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if let Some(draft) = ctx.weapon.draft.as_deref_mut() {
                        weapon_form_ui::def_panel(ui, draft, ctx.weapon.attachments.as_deref());
                    }
                });
        }
        EditorMode::MeleeWeapon => {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if let Some(draft) = ctx.melee_weapon.draft.as_deref_mut() {
                        melee_weapon_form_ui::def_panel(
                            ui,
                            draft,
                            ctx.melee_weapon.attachments.as_deref(),
                        );
                    }
                });
        }
    });
}

fn injury_stack(
    ui: &mut egui::Ui,
    ctx: &mut ModePanelsCtx<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
) {
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
                    ctx.last_save,
                );
            }
        });
}

fn prefab_viewport(
    ui: &mut egui::Ui,
    ctx: &mut ModePanelsCtx<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
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
