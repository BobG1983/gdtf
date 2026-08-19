use bevy::{ecs::message::Messages, image::Image, prelude::*};
use gdtf_battle_sim::{
    cover::CoverLedger,
    occupancy_sync::TerrainPieceDestroyed,
    prelude::{BattleInProgress, OccupancyGrid},
    surface::SurfaceGrid,
    vertical::VerticalLinkGraph,
};
use gdtf_content_families::sprites::SpriteDefRegistry;

use crate::{
    MissingTileTexture, PresenterSystems, draw_static_battlefield, draw_vertical_links,
    indicate_emplacement_occupied, render::terrain::setup_missing_tile_texture,
    restamp_tiles_on_def_change, swap_destroyed_cover, swap_destroyed_slab,
};

pub(super) fn register_terrain_draw(app: &mut App) {
    app.add_systems(
        Startup,
        setup_missing_tile_texture.run_if(resource_exists::<Assets<Image>>),
    );
    app.add_systems(
        Update,
        draw_static_battlefield
            .in_set(PresenterSystems::Scene)
            .run_if(
                resource_exists::<BattleInProgress>
                    .and_then(resource_exists::<SpriteDefRegistry>)
                    .and_then(resource_exists::<AssetServer>)
                    .and_then(resource_exists::<MissingTileTexture>)
                    .and_then(resource_exists::<OccupancyGrid>)
                    .and_then(resource_exists::<CoverLedger>)
                    .and_then(resource_exists::<SurfaceGrid>),
            ),
    );
}

pub(super) fn register_destruction_swaps(app: &mut App) {
    app.add_systems(
        Update,
        swap_destroyed_cover
            .in_set(PresenterSystems::Scene)
            .after(swap_destroyed_slab)
            .run_if(resource_exists::<BattleInProgress>.and_then(sprite_resolution_ready)),
    )
    .add_systems(
        Update,
        swap_destroyed_slab.in_set(PresenterSystems::Scene).run_if(
            resource_exists::<BattleInProgress>
                .and_then(sprite_resolution_ready)
                .and_then(resource_exists::<Messages<TerrainPieceDestroyed>>),
        ),
    )
    .add_systems(
        Update,
        indicate_emplacement_occupied
            .in_set(PresenterSystems::Scene)
            .run_if(resource_exists::<BattleInProgress>.and_then(sprite_resolution_ready)),
    )
    .add_systems(
        Update,
        restamp_tiles_on_def_change
            .in_set(PresenterSystems::Scene)
            .after(draw_static_battlefield)
            .after(swap_destroyed_cover)
            .after(swap_destroyed_slab)
            .after(indicate_emplacement_occupied)
            .run_if(resource_exists::<BattleInProgress>.and_then(sprite_resolution_ready)),
    );
}

const fn sprite_resolution_ready(
    defs: Option<Res<SpriteDefRegistry>>,
    asset_server: Option<Res<AssetServer>>,
    missing: Option<Res<MissingTileTexture>>,
) -> bool {
    defs.is_some() && asset_server.is_some() && missing.is_some()
}

pub(super) fn register_vertical_links(app: &mut App) {
    app.add_systems(
        Update,
        draw_vertical_links.in_set(PresenterSystems::Scene).run_if(
            resource_exists::<BattleInProgress>
                .and_then(resource_exists::<VerticalLinkGraph>)
                .and_then(sprite_resolution_ready),
        ),
    );
}
