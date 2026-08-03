//! Three unmistakable per-cell classes at any zoom (GTW-594 C2): AUTHORED-HERE (active
use bevy::prelude::*;
use gdtf_battle_presenter::{StoreyTreatment, anchor_world_offset, source_parts, source_px_size};
use gdtf_battle_sim::{
    metric::{CellLevel, Level},
    prelude::Cell,
    terrain::def::TerrainDefRegistry,
};
use gdtf_content_families::sprites::{SpriteDef, SpriteDefRegistry};

use crate::{
    editor_map::EditorMap,
    hovered_cell::HoveredCell,
    placement::{ProposedPlacement, evaluate_placement},
    preview::{
        coords::{CELL_WORLD, cell_center_world},
        target::{PreviewTile, preview_layer},
    },
    session::MapEditorSession,
    terrain_graphics::terrain_sprite_def,
};

pub(super) const TILE_Z: f32 = 0.0;

pub(super) const STOREY_Z_GAP: f32 = 0.01;

pub(super) const OVERLAY_Z_LIFT: f32 = STOREY_Z_GAP * 0.25;

pub(super) const GHOST_Z: f32 = 1.0;

const GHOST_LEGAL: Color = Color::srgba(1.0, 1.0, 1.0, 0.45);

const GHOST_ILLEGAL: Color = Color::srgba(1.0, 0.2, 0.2, 0.55);

pub(super) const GHOST_BELOW_TINT: Color = Color::srgba(0.55, 0.66, 0.82, 0.55);

pub(super) const STIPPLE_TINT: Color = Color::srgba(0.35, 0.5, 0.75, 0.4);

pub(super) const VOID_GRID_TINT: Color = Color::srgba(0.65, 0.68, 0.75, 0.22);

pub(super) const MISSING_SPRITE_TINT: Color = Color::srgb(0.78, 0.24, 0.78);

pub(super) const fn base_tile_tint(treatment: StoreyTreatment) -> Option<Color> {
    match treatment {
        StoreyTreatment::Hidden => None,
        StoreyTreatment::Active => Some(Color::WHITE),
        StoreyTreatment::ContextBelow(_) => Some(GHOST_BELOW_TINT),
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "the ghost needs the same model + registry inputs the base redraw resolved (map, \
              defs, sprites, session, hover) plus the asset server + Commands to spawn one \
              sprite; each is a borrowed SystemParam slice threaded from the single redraw system"
)]
pub(super) fn draw_hover_ghost(
    commands: &mut Commands,
    asset_server: &AssetServer,
    map: &EditorMap,
    registry: &TerrainDefRegistry,
    sprites: &SpriteDefRegistry,
    session: &MapEditorSession,
    hovered: &HoveredCell,
    level: Level,
) {
    let Some(cell) = hovered.cell() else {
        return;
    };
    let Some(tile) = session.selected_tile() else {
        return;
    };
    let slot = CellLevel::new(cell, level);
    let placement = ProposedPlacement::new(slot, tile);
    let verdict = evaluate_placement(
        map,
        registry,
        session.theme(),
        &placement,
        session.grid_size(),
    );
    let tint = if verdict.is_illegal() {
        GHOST_ILLEGAL
    } else {
        GHOST_LEGAL
    };
    match terrain_sprite_def(registry, sprites, &tile) {
        Some(def) => spawn_tile_sprite(commands, asset_server, cell, def, tint, GHOST_Z),
        None => spawn_color_tile(commands, cell, tint, GHOST_Z),
    }
}

pub(super) fn spawn_tile_sprite(
    commands: &mut Commands,
    asset_server: &AssetServer,
    cell: Cell,
    def: &SpriteDef,
    tint: Color,
    z: f32,
) {
    let (path, rect) = source_parts(&def.source);
    let mut sprite = Sprite::from_image(asset_server.load(path.as_str().to_owned()));
    sprite.rect = rect.map(|rect| gdtf_battle_presenter::source_urect(rect).as_rect());
    sprite.custom_size = Some(Vec2::splat(CELL_WORLD));
    sprite.color = tint;
    let offset = source_px_size(&def.source).map_or(Vec2::ZERO, |px| {
        anchor_world_offset(def, px, Vec2::splat(CELL_WORLD))
    });
    spawn_preview_sprite_offset(commands, sprite, cell, offset, z);
}

pub(super) fn spawn_color_tile(commands: &mut Commands, cell: Cell, tint: Color, z: f32) {
    let mut sprite = Sprite::from_color(tint, Vec2::splat(CELL_WORLD));
    sprite.custom_size = Some(Vec2::splat(CELL_WORLD));
    spawn_preview_sprite(commands, sprite, cell, z);
}

pub(super) fn spawn_overlay_sprite(
    commands: &mut Commands,
    image: Handle<bevy::image::Image>,
    cell: Cell,
    tint: Color,
    z: f32,
) {
    let mut sprite = Sprite::from_image(image);
    sprite.custom_size = Some(Vec2::splat(CELL_WORLD));
    sprite.color = tint;
    spawn_preview_sprite(commands, sprite, cell, z);
}

fn spawn_preview_sprite(commands: &mut Commands, sprite: Sprite, cell: Cell, z: f32) {
    spawn_preview_sprite_offset(commands, sprite, cell, Vec2::ZERO, z);
}

fn spawn_preview_sprite_offset(
    commands: &mut Commands,
    sprite: Sprite,
    cell: Cell,
    offset: Vec2,
    z: f32,
) {
    let world = cell_center_world(cell) + offset;
    commands.spawn((
        sprite,
        Transform::from_translation(world.extend(z)),
        preview_layer(),
        PreviewTile,
    ));
}
