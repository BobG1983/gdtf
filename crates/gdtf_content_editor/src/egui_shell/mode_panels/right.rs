//! The RIGHT mode-form panel dispatch — one arm per Workbench mode (moved out of
//! `shell.rs` at the GTW-670 band boundary: this dispatch grows an arm per mode, the shell
//! only changes when the panel LAYOUT does). The bodies are the per-mode `*_form_ui`
//! modules; this file owns only the branch.

use bevy_egui::egui;

use super::ctx::ModePanelsCtx;
use crate::{
    egui_shell::{
        armor_form_ui, attachment_form_ui, gang_form_ui, injury_form_ui, melee_weapon_form_ui,
        prefab::controls_ui, sprite_form_ui, theme_form_ui, weapon_form_ui,
    },
    mode::EditorMode,
};

/// Declare the RIGHT panel — the ACTIVE mode's form (an in-UI branch; the panel-4 slot
/// of the shell's load-bearing outermost-first order). TERRAIN no longer renders here
/// (GTW-534 C1 folded its picker + field stack into the CENTRAL primary region); THEME
/// is the real form (GTW-514); PREFAB the real controls (GTW-515) — the grid-size
/// fields, the GTW-595 level rail, and the debug Save; the GANG / ARMOR / INJURY /
/// SPRITE / ATTACHMENT / WEAPON / MELEE modes each draw their `fields.rs` stack (load /
/// name / New / debug Save).
pub(in crate::egui_shell) fn right_panel(
    viewport_ui: &mut egui::Ui,
    mode: EditorMode,
    ctx: &mut ModePanelsCtx<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
) {
    egui::Panel::right("editor_mode_form").show(viewport_ui, |ui| match mode {
        // TERRAIN's controls are central now (GTW-534 C1); the right panel is
        // intentionally idle in TERRAIN mode so nothing competes with the central
        // stats + sprite picker.
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
                    ctx.session,
                    edit_level,
                    view,
                    isolate,
                    &mut *ctx.prefab_save_name,
                    map,
                    ctx.terrain_registry,
                    ctx.themes,
                    &mut ctx.prefab.rail_state,
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
        // GTW-664: load / name / New sprite / debug Save — the Gang/Armor field-stack
        // parity over the sprite draft + the GTW-663 registry.
        EditorMode::Sprite => {
            if let Some(draft) = ctx.sprite.draft.as_deref_mut() {
                sprite_form_ui::field_stack(ui, draft, ctx.sprite.registry.as_deref());
            }
        }
        // GTW-669: load / name / New attachment / debug Save — the Gang/Armor
        // field-stack parity over the attachment draft + the GTW-619 registry.
        EditorMode::Attachment => {
            if let Some(draft) = ctx.attachment.draft.as_deref_mut() {
                attachment_form_ui::field_stack(ui, draft, ctx.attachment.registry.as_deref());
            }
        }
        // GTW-670: load / name / New weapon / debug Save — the Gang/Armor field-stack
        // parity over the weapon draft + the GTW-257 registry.
        EditorMode::Weapon => {
            if let Some(draft) = ctx.weapon.draft.as_deref_mut() {
                weapon_form_ui::field_stack(ui, draft, ctx.weapon.registry.as_deref());
            }
        }
        // GTW-671: load / name / New melee weapon / debug Save — the Weapon field-stack
        // parity over the melee draft + the GTW-505 registry.
        EditorMode::MeleeWeapon => {
            if let Some(draft) = ctx.melee_weapon.draft.as_deref_mut() {
                melee_weapon_form_ui::field_stack(ui, draft, ctx.melee_weapon.registry.as_deref());
            }
        }
    });
}
