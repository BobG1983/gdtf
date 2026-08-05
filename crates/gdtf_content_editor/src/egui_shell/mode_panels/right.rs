use bevy_egui::egui;

use super::ctx::ModePanelsCtx;
use crate::{
    egui_shell::{
        armor_form_ui, attachment_form_ui, gang_form_ui, injury_form_ui, melee_weapon_form_ui,
        prefab::controls_ui::{self, EditedPrefab, StoreyToggles, TerrainLibrary},
        sprite_form_ui, theme_form_ui, weapon_form_ui,
    },
    mode::EditorMode,
};

pub(in crate::egui_shell) fn right_panel(
    viewport_ui: &mut egui::Ui,
    mode: EditorMode,
    ctx: &mut ModePanelsCtx<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
) {
    egui::Panel::right("editor_mode_form").show(viewport_ui, |ui| match mode {
        EditorMode::Terrain => {}
        EditorMode::Theme => {
            theme_form_ui::field_stack(ui, ctx.theme_draft, ctx.terrain_registry);
        }
        EditorMode::Prefab => {
            if let (Some(edit_level), Some(view), Some(isolate), Some(map)) = (
                ctx.prefab.edit_level.as_deref_mut(),
                ctx.prefab.view.as_deref_mut(),
                ctx.prefab.isolate.as_deref_mut(),
                ctx.prefab.map.as_deref(),
            ) {
                controls_ui::controls_panel(
                    ui,
                    EditedPrefab {
                        map,
                        session: ctx.session,
                        edit_level,
                    },
                    StoreyToggles { view, isolate },
                    TerrainLibrary {
                        terrain: ctx.terrain_registry,
                        themes:  ctx.themes,
                    },
                    &mut ctx.prefab.rail_state,
                    &mut ctx.prefab.save_name,
                );
            }
        }
        EditorMode::Gang => {
            if let Some(draft) = ctx.gang.draft.as_deref_mut() {
                gang_form_ui::field_stack(ui, draft, ctx.gang.gangs.as_deref());
            }
        }
        EditorMode::Armor => {
            if let Some(draft) = ctx.armor.draft.as_deref_mut() {
                armor_form_ui::field_stack(ui, draft, ctx.armor.registry.as_deref());
            }
        }
        EditorMode::Injury => {
            if let Some(draft) = ctx.injury.draft.as_deref_mut() {
                injury_form_ui::field_stack(ui, draft, ctx.injury.registry.as_deref());
            }
        }
        EditorMode::Sprite => {
            if let Some(draft) = ctx.sprite.draft.as_deref_mut() {
                sprite_form_ui::field_stack(ui, draft, ctx.sprite.registry.as_deref());
            }
        }
        EditorMode::Attachment => {
            if let Some(draft) = ctx.attachment.draft.as_deref_mut() {
                attachment_form_ui::field_stack(ui, draft, ctx.attachment.registry.as_deref());
            }
        }
        EditorMode::Weapon => {
            if let Some(draft) = ctx.weapon.draft.as_deref_mut() {
                weapon_form_ui::field_stack(ui, draft, ctx.weapon.registry.as_deref());
            }
        }
        EditorMode::MeleeWeapon => {
            if let Some(draft) = ctx.melee_weapon.draft.as_deref_mut() {
                melee_weapon_form_ui::field_stack(ui, draft, ctx.melee_weapon.registry.as_deref());
            }
        }
    });
}
