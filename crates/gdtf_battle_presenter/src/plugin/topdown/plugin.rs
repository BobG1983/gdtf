//! Top-down renderer plugin wiring.

use bevy::{prelude::*, sprite_render::Material2dPlugin};
use gdtf_battle_sim::occupancy_sync::SimSystems;

use crate::{
    ActiveLevel, FireTargetHighlight, GangerSprites, IsolateView, PathPreview, PresenterSystems,
    TerrainFogMaterial, ViewMode,
    actors::{
        fx::{register_effect_roles_hot_ron, register_fx_tuning_hot_ron},
        ganger::register_character_roles_hot_ron,
    },
    load_topdown_atlases,
    render::{topdown::register_sheet_image_redrive, world_camera::register_pan_tuning_hot_ron},
};

/// Marker resource present while the top-down renderer is active.
#[derive(Resource)]
pub struct TopDownRendererActive;

/// Installs top-down draw, playback, overlays, FX, and camera systems.
pub struct TopDownRendererPlugin;

impl Plugin for TopDownRendererPlugin {
    fn build(&self, app: &mut App) {
        if app.world().get_resource::<AssetServer>().is_some() {
            app.add_plugins(Material2dPlugin::<TerrainFogMaterial>::default());
        }
        app.insert_resource(TopDownRendererActive)
            .init_resource::<ActiveLevel>()
            .init_resource::<ViewMode>()
            .init_resource::<IsolateView>()
            .init_resource::<PathPreview>()
            .init_resource::<FireTargetHighlight>()
            .init_resource::<GangerSprites>()
            .add_systems(
                Startup,
                load_topdown_atlases
                    .run_if(resource_exists::<Assets<bevy::image::TextureAtlasLayout>>),
            );

        register_hot_ron_chains(app);

        app.configure_sets(Update, PresenterSystems::Draw.after(SimSystems::Record));
        app.configure_sets(
            Update,
            (
                PresenterSystems::Replay,
                PresenterSystems::Scene,
                PresenterSystems::Compose,
                PresenterSystems::Overlay,
            )
                .chain()
                .in_set(PresenterSystems::Draw),
        );

        crate::playback::register_playback(app);

        super::terrain::register_terrain_draw(app);

        super::terrain::register_destruction_swaps(app);

        super::terrain::register_vertical_links(app);

        super::gangers::register_ganger_draw(app);

        super::fx::register_fx_flash_systems(app);

        super::camera::register_camera_framing_systems(app);

        super::overlays::register_highlight_systems(app);

        super::fog::register_fog_systems(app);

        super::overlays::register_path_preview_systems(app);

        super::overlays::register_field_overlay_systems(app);

        super::fx::register_consequence_fct_families(app);

        super::combat_log::register_combat_log_forwarders(app);

        super::overlays::register_reachable_overlay_systems(app);

        super::overlays::register_fire_target_systems(app);

        super::overlays::register_cross_level_signals_systems(app);
    }
}

fn register_hot_ron_chains(app: &mut App) {
    register_character_roles_hot_ron(app);
    register_effect_roles_hot_ron(app);
    register_fx_tuning_hot_ron(app);
    register_pan_tuning_hot_ron(app);
    register_sheet_image_redrive(app);
}
