use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::{
    egui_shell::{
        autoload::{ModeSyncBundles, run_form_syncs},
        chrome::{mode_tabs, status_line, theme_combo_box},
        mode_panels::{ModePanelsCtx, central_panel, right_panel},
        params::{
            ArmorParams, AttachmentParams, GangParams, InjuryParams, MeleeWeaponParams,
            PrefabParams, SharedRegistries, SpriteParams, WeaponParams,
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

#[expect(
    clippy::too_many_arguments,
    reason = "the whole-editor egui system draws ALL panels in one pass (the EguiPrimaryContextPass \
              requirement — bevy-traps #8); each param is a distinct Bevy SystemParam (the egui \
              context, the state-scoped mutable drafts + mode + session, the per-mode model \
              bundles — prefab / gang / armor / injury / sprite / attachment / weapon / melee — \
              and the read-only registries); Bevy's injection model cannot reduce this without a \
              wrapper resource that changes the crate's API surface"
)]
pub(crate) fn editor_egui_ui(
    mut contexts: EguiContexts,
    mut prefab_save_name: Local<String>,
    mode: Option<ResMut<EditorMode>>,
    session: Option<ResMut<MapEditorSession>>,
    terrain_draft: Option<ResMut<TerrainDraft>>,
    theme_draft: Option<ResMut<ThemeDraft>>,
    shared: SharedRegistries,
    mut prefab: PrefabParams,
    mut gang: GangParams,
    mut armor_mode: ArmorParams,
    mut injury_mode: InjuryParams,
    mut sprite_mode: SpriteParams,
    mut attachment_mode: AttachmentParams,
    mut weapon_mode: WeaponParams,
    mut melee_weapon_mode: MeleeWeaponParams,
) -> Result {
    let (Some(mut mode), Some(mut session), Some(mut terrain_draft), Some(mut theme_draft)) =
        (mode, session, terrain_draft, theme_draft)
    else {
        return Ok(());
    };

    run_form_syncs(
        *mode,
        &session,
        shared.themes.as_deref(),
        &mut theme_draft,
        ModeSyncBundles {
            gang:         &mut gang,
            armor:        &mut armor_mode,
            injury:       &mut injury_mode,
            sprite:       &mut sprite_mode,
            attachment:   &mut attachment_mode,
            weapon:       &mut weapon_mode,
            melee_weapon: &mut melee_weapon_mode,
        },
    );

    let textures = resolve_panel_textures(&mut contexts, *mode, &prefab, &mut sprite_mode);

    let ctx = contexts.ctx_mut()?;
    let mut viewport_ui = egui::Ui::new(
        ctx.clone(),
        "editor_viewport".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );
    let options = theme_options(shared.themes.as_deref());

    egui::Panel::top("editor_top_bar").show(&mut viewport_ui, |ui| {
        ui.horizontal(|ui| {
            mode_tabs(ui, &mut mode);
            ui.separator();
            theme_combo_box(ui, &options, shared.themes.as_deref(), &mut session);
        });
    });

    egui::Panel::bottom("editor_status_bar").show(&mut viewport_ui, |ui| {
        ui.label(status_line(*mode, &session, shared.themes.as_deref()));
    });

    egui::Panel::left("editor_palette").show(&mut viewport_ui, |ui| match *mode {
        EditorMode::Terrain => {
            terrain_form_ui::ron_preview(ui, &terrain_draft);
        }
        EditorMode::Theme => {
            theme_form_ui::stats_panel(ui, &theme_draft, shared.terrain.as_deref());
        }
        EditorMode::Prefab => {
            palette_ui::palette_panel(
                ui,
                &mut session,
                shared.themes.as_deref(),
                shared.terrain.as_deref(),
                sprite_mode.registry.as_deref(),
                &textures.sprites,
            );
        }
        EditorMode::Gang
        | EditorMode::Armor
        | EditorMode::Injury
        | EditorMode::Sprite
        | EditorMode::Attachment
        | EditorMode::Weapon
        | EditorMode::MeleeWeapon => {}
    });

    let mut panel_ctx = ModePanelsCtx {
        session:          &mut session,
        terrain_draft:    &mut terrain_draft,
        theme_draft:      &mut theme_draft,
        prefab_save_name: &mut prefab_save_name,
        themes:           shared.themes.as_deref(),
        terrain_registry: shared.terrain.as_deref(),
        weapons:          shared.weapons.as_deref(),
        textures:         &textures,
        prefab:           &mut prefab,
        gang:             &mut gang,
        armor:            &mut armor_mode,
        injury:           &mut injury_mode,
        sprite:           &mut sprite_mode,
        attachment:       &mut attachment_mode,
        weapon:           &mut weapon_mode,
        melee_weapon:     &mut melee_weapon_mode,
    };
    right_panel(&mut viewport_ui, *mode, &mut panel_ctx);
    central_panel(&mut viewport_ui, *mode, &mut panel_ctx);

    Ok(())
}
