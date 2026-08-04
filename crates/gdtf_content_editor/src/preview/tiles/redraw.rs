use bevy::prelude::*;
use gdtf_battle_presenter::{
    ActiveLevel, IsolateView, StoreyTreatment, StoreyViewMode, ViewMode, storey_treatment,
};
use gdtf_battle_sim::{
    level::{GridSize, UuidThemeRegistry},
    metric::{CellLevel, Level},
    prelude::Cell,
    terrain::def::{TerrainDefRegistry, TerrainUuid},
};
use gdtf_content_families::sprites::SpriteDefRegistry;

use super::sprites::{
    MISSING_SPRITE_TINT, OVERLAY_Z_LIFT, STIPPLE_TINT, STOREY_Z_GAP, TILE_Z, VOID_GRID_TINT,
    base_tile_tint, draw_hover_ghost, spawn_color_tile, spawn_overlay_sprite, spawn_tile_sprite,
};
use crate::{
    canvas::CurrentEditLevel,
    editor_map::EditorMap,
    hovered_cell::HoveredCell,
    preview::{overlay::PreviewOverlayImages, target::PreviewTile},
    session::MapEditorSession,
    terrain_graphics::terrain_sprite_def,
};

const GROUND_STOREY: u8 = 0;

#[expect(
    clippy::too_many_arguments,
    reason = "the preview redraw reads every model input it depends on (map, session, edit level, \
              hover, view mode, isolate toggle) + the three shared registries (terrain defs, \
              themes, sprite defs) + the asset server + the generated overlay textures + Commands \
              to (re)spawn; each is a distinct Bevy SystemParam and Bevy's injection cannot reduce \
              them without a wrapper resource that changes the crate API"
)]
pub(crate) fn redraw_preview_tiles(
    mut commands: Commands,
    map: Option<Res<EditorMap>>,
    session: Option<Res<MapEditorSession>>,
    edit_level: Option<Res<CurrentEditLevel>>,
    hovered: Option<Res<HoveredCell>>,
    view: Option<Res<ViewMode>>,
    isolate: Option<Res<IsolateView>>,
    overlays: Option<Res<PreviewOverlayImages>>,
    registry: Option<Res<TerrainDefRegistry>>,
    themes: Option<Res<UuidThemeRegistry>>,
    sprites: Option<Res<SpriteDefRegistry>>,
    asset_server: Option<Res<AssetServer>>,
    existing: Query<Entity, With<PreviewTile>>,
) {
    let (
        Some(map),
        Some(session),
        Some(edit_level),
        Some(hovered),
        Some(view),
        Some(isolate),
        Some(overlays),
        Some(registry),
        Some(themes),
        Some(sprites),
        Some(asset_server),
    ) = (
        map,
        session,
        edit_level,
        hovered,
        view,
        isolate,
        overlays,
        registry,
        themes,
        sprites,
        asset_server,
    )
    else {
        return;
    };

    let dirty = map.is_changed()
        || session.is_changed()
        || edit_level.is_changed()
        || hovered.is_changed()
        || view.is_changed()
        || isolate.is_changed()
        || overlays.is_changed()
        || registry.is_changed()
        || themes.is_changed()
        || sprites.is_changed();
    if !dirty {
        return;
    }

    for entity in &existing {
        commands.entity(entity).despawn();
    }

    let level = edit_level.level();
    let size = session.grid_size();
    let theme = session.theme();
    let pass = StoreyPass {
        map: &map,
        registry: &registry,
        sprites: &sprites,
        asset_server: &asset_server,
        overlays: &overlays,
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

    draw_hover_ghost(
        &mut commands,
        pass.asset_server,
        &map,
        &registry,
        &sprites,
        &session,
        &hovered,
        level,
    );
}

struct StoreyPass<'a> {
    map:           &'a EditorMap,
    registry:      &'a TerrainDefRegistry,
    sprites:       &'a SpriteDefRegistry,
    asset_server:  &'a AssetServer,
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
            let tile = if storey == GROUND_STOREY {
                pass.map.tile_at_level(slot).or(pass.default_floor)
            } else {
                pass.map.tile_at_level(slot)
            };
            let Some(tile) = tile else {
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
            match terrain_sprite_def(pass.registry, pass.sprites, &tile) {
                Some(def) => {
                    spawn_tile_sprite(commands, pass.asset_server, cell, def, tint, z);
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
