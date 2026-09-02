use bevy::prelude::*;
use gdtf_battle_presenter::{ActiveLevel, StoreyTreatment, StoreyViewMode, storey_treatment};
use gdtf_battle_sim::{
    level::GridSize,
    metric::{CellLevel, Level},
    prelude::Cell,
    terrain::{def::TerrainUuid, facing::TerrainFacing},
};

use super::{
    sources::{PreviewSubject, StoreyVisibility, TileArt},
    sprites::{
        MISSING_SPRITE_TINT, OVERLAY_Z_LIFT, STIPPLE_TINT, STOREY_Z_GAP, TILE_Z, TileArtwork,
        VOID_GRID_TINT, base_tile_tint, draw_hover_ghost, spawn_color_tile, spawn_overlay_sprite,
        spawn_tile_sprite,
    },
};
use crate::{
    editor_map::EditorMap,
    preview::{overlay::PreviewOverlayImages, target::PreviewTile},
    terrain_graphics::terrain_sprite_def,
};

const GROUND_STOREY: u8 = 0;

pub(crate) fn redraw_preview_tiles(
    mut commands: Commands,
    subject: PreviewSubject,
    storeys: StoreyVisibility,
    art: TileArt,
    existing: Query<Entity, With<PreviewTile>>,
) {
    let (Some(map), Some(session), Some(hovered)) = (
        subject.map.as_deref(),
        subject.session.as_deref(),
        subject.hovered.as_deref(),
    ) else {
        return;
    };
    let (Some(edit_level), Some(view), Some(isolate)) = (
        storeys.edit_level.as_deref(),
        storeys.view.as_deref(),
        storeys.isolate.as_deref(),
    ) else {
        return;
    };
    let (Some(registry), Some(themes), Some(sprites), Some(overlays), Some(asset_server)) = (
        art.terrain.as_deref(),
        art.themes.as_deref(),
        art.sprites.as_deref(),
        art.overlays.as_deref(),
        art.asset_server.as_deref(),
    ) else {
        return;
    };

    let dirty = subject.changed() | storeys.changed() | art.changed();
    if !*dirty {
        return;
    }

    for entity in &existing {
        commands.entity(entity).despawn();
    }

    let level = edit_level.level();
    let size = session.grid_size();
    let theme = session.theme();
    let pass = StoreyPass {
        map,
        art: TileArtwork {
            registry,
            sprites,
            assets: asset_server,
        },
        overlays,
        default_floor: session
            .default_floor()
            .or_else(|| themes.default_floor(&theme)),
        size,
    };

    let active = ActiveLevel::new(level);
    let mode = StoreyViewMode::new(*view, *isolate);
    for storey in 0..*size.levels() {
        let treatment = storey_treatment(Level::new(storey), active, mode);
        draw_storey(&mut commands, &pass, storey, treatment);
    }

    draw_hover_ghost(&mut commands, &pass.art, map, session, hovered, level);
}

struct StoreyPass<'a> {
    map:           &'a EditorMap,
    art:           TileArtwork<'a>,
    overlays:      &'a PreviewOverlayImages,
    default_floor: Option<TerrainUuid>,
    size:          GridSize,
}

fn draw_storey(
    commands: &mut Commands,
    pass: &StoreyPass<'_>,
    storey: u8,
    treatment: StoreyTreatment,
) {
    let Some(tint) = base_tile_tint(treatment) else {
        return;
    };
    let is_active_storey = treatment == StoreyTreatment::Active;
    let is_context_storey = matches!(treatment, StoreyTreatment::ContextBelow(_));
    let storey_level = Level::new(storey);
    let z = STOREY_Z_GAP.mul_add(f32::from(storey), TILE_Z);
    for y in 0..i32::from(*pass.size.height()) {
        for x in 0..i32::from(*pass.size.width()) {
            let cell = Cell::new(x, y);
            let slot = CellLevel::new(cell, storey_level);
            let painted = pass.map.tile_at_level(slot);
            let tile = if storey == GROUND_STOREY {
                painted
                    .map(|piece| (piece.tile(), piece.facing()))
                    .or_else(|| {
                        pass.default_floor
                            .map(|floor| (floor, TerrainFacing::default()))
                    })
            } else {
                painted.map(|piece| (piece.tile(), piece.facing()))
            };
            let Some((tile, facing)) = tile else {
                if is_active_storey {
                    spawn_overlay_sprite(
                        commands,
                        pass.overlays.void_grid(),
                        cell,
                        VOID_GRID_TINT,
                        z,
                    );
                }
                continue;
            };
            match terrain_sprite_def(pass.art.registry, pass.art.sprites, &tile, facing) {
                Some(def) => {
                    spawn_tile_sprite(commands, pass.art.assets, cell, def, tint, z);
                }
                None => spawn_color_tile(commands, cell, MISSING_SPRITE_TINT, z),
            }
            if is_context_storey {
                spawn_overlay_sprite(
                    commands,
                    pass.overlays.stipple(),
                    cell,
                    STIPPLE_TINT,
                    z + OVERLAY_Z_LIFT,
                );
            }
        }
    }
}
