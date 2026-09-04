use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::{
    egui_shell::{
        autoload::{ModeSyncBundles, run_form_syncs},
        chrome::chrome_bars,
        mode_panels::{ModePanelsCtx, central_panel, right_panel},
        params::{ContentForms, PrefabParams, SharedRegistries, TerrainThemeDrafts, Workbench},
        prefab::palette_ui,
        terrain_form_ui,
        textures::resolve_panel_textures,
        theme_combo::theme_options,
        theme_form_ui,
    },
    mode::EditorMode,
};

pub(crate) fn editor_egui_ui(
    mut contexts: EguiContexts,
    mut workbench: Workbench,
    drafts: TerrainThemeDrafts,
    shared: SharedRegistries,
    mut prefab: PrefabParams,
    mut forms: ContentForms,
) -> Result {
    let TerrainThemeDrafts { terrain, theme } = drafts;
    let (Some(mut mode), Some(mut session), Some(mut terrain_draft), Some(mut theme_draft)) = (
        workbench.mode.take(),
        workbench.session.take(),
        terrain,
        theme,
    ) else {
        return Ok(());
    };
    let last_save = &mut *workbench.last_save;

    run_form_syncs(
        *mode,
        &session,
        shared.themes.as_deref(),
        &mut theme_draft,
        shared.theme_source.as_deref(),
        ModeSyncBundles {
            gang:         &mut forms.gang,
            armor:        &mut forms.armor,
            injury:       &mut forms.injury,
            sprite:       &mut forms.sprite,
            attachment:   &mut forms.attachment,
            weapon:       &mut forms.weapon,
            melee_weapon: &mut forms.melee_weapon,
            field:        &mut forms.field,
        },
    );

    let textures = resolve_panel_textures(&mut contexts, *mode, &prefab, &mut forms.sprite);

    let ctx = contexts.ctx_mut()?;
    let mut viewport_ui = egui::Ui::new(
        ctx.clone(),
        "editor_viewport".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );
    chrome_bars(
        &mut viewport_ui,
        &mut mode,
        &mut session,
        &theme_options(shared.themes.as_deref()),
        shared.themes.as_deref(),
    );

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
                forms.sprite.registry.as_deref(),
                &textures.sprites,
            );
        }
        EditorMode::Gang
        | EditorMode::Armor
        | EditorMode::Injury
        | EditorMode::Sprite
        | EditorMode::Attachment
        | EditorMode::Weapon
        | EditorMode::MeleeWeapon
        | EditorMode::Field => {}
    });

    #[cfg(feature = "mcp")]
    let mut delete_request = None;
    let mut panel_ctx = ModePanelsCtx {
        session: &mut session,
        last_save,
        terrain_draft: &mut terrain_draft,
        theme_draft: &mut theme_draft,
        themes: shared.themes.as_deref(),
        terrain_registry: shared.terrain.as_deref(),
        terrain_sources: shared.terrain_source.as_deref(),
        weapons: shared.weapons.as_deref(),
        textures: &textures,
        prefab: &mut prefab,
        gang: &mut forms.gang,
        armor: &mut forms.armor,
        injury: &mut forms.injury,
        sprite: &mut forms.sprite,
        attachment: &mut forms.attachment,
        weapon: &mut forms.weapon,
        melee_weapon: &mut forms.melee_weapon,
        field: &mut forms.field,
        #[cfg(feature = "mcp")]
        deletes: workbench.deletes.as_deref(),
        #[cfg(feature = "mcp")]
        delete_request: &mut delete_request,
    };
    right_panel(&mut viewport_ui, *mode, &mut panel_ctx);
    central_panel(&mut viewport_ui, *mode, &mut panel_ctx);

    #[cfg(feature = "mcp")]
    insert_delete_request(&mut workbench.commands, delete_request);

    Ok(())
}

// Ask for the delete the drawn control answered with, if the author pressed it.
#[cfg(feature = "mcp")]
fn insert_delete_request(commands: &mut Commands, request: Option<crate::delete::DeleteRequest>) {
    if let Some(request) = request {
        commands.insert_resource(request);
    }
}
