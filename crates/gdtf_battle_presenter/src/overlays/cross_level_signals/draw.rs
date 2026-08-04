//! Draw cross-level threat, drop-depth, and connector badges.

use bevy::{camera::visibility::RenderLayers, prelude::*, sprite::Anchor, text::FontSize};
use gdtf_battle_sim::prelude::{Cell, Level};

use super::types::{CrossLevelBadgeKind, CrossLevelSignals};
use crate::{
    ActiveLevel, Layer, WORLD_RENDER_LAYER, cell_to_world_layered, overlays::pool::draw_pool,
};

/// Marker on a cross-level badge tile sprite.
#[derive(Component, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct CrossLevelBadgeTile;

/// Marker on a cross-level badge label.
#[derive(Component, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct CrossLevelBadgeLabel;

type TileQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Sprite,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    (With<CrossLevelBadgeTile>, Without<CrossLevelBadgeLabel>),
>;

type LabelQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Text2d,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    (With<CrossLevelBadgeLabel>, Without<CrossLevelBadgeTile>),
>;

const BADGE_SIZE_PX: f32 = 6.0;

const BADGE_LABEL_FONT_PX: f32 = 5.0;

const LABEL_Z_LIFT: f32 = 0.001;

const BADGE_SLOT_OFFSETS: [Vec2; 3] = [
    Vec2::new(5.0, 5.0),
    Vec2::new(5.0, -1.0),
    Vec2::new(5.0, -7.0),
];

const THREAT_TINT: Color = Color::srgb(0.85, 0.2, 0.2);
const DROP_DEPTH_TINT: Color = Color::srgb(0.6, 0.42, 0.15);
const CONNECTOR_TINT: Color = Color::srgb(0.25, 0.55, 0.9);
const BADGE_LABEL_COLOR: Color = Color::srgb(0.95, 0.97, 0.95);

#[derive(Debug, Clone, PartialEq)]
struct BadgeDraw {
    world: Vec3,
    tint:  Color,
    label: String,
}

const fn badge_tint(kind: CrossLevelBadgeKind) -> Color {
    match kind {
        CrossLevelBadgeKind::Threat { .. } => THREAT_TINT,
        CrossLevelBadgeKind::DropDepth { .. } => DROP_DEPTH_TINT,
        CrossLevelBadgeKind::ConnectorDelta { .. } => CONNECTOR_TINT,
    }
}

fn badge_label(kind: CrossLevelBadgeKind) -> String {
    match kind {
        CrossLevelBadgeKind::Threat { delta, count } => {
            let sign = if delta.is_above() { '+' } else { '-' };
            if *count > 1 {
                format!("{sign}{} x{}", delta.magnitude(), *count)
            } else {
                format!("{sign}{}", delta.magnitude())
            }
        }
        CrossLevelBadgeKind::DropDepth { storeys } => format!("v{}", *storeys),
        CrossLevelBadgeKind::ConnectorDelta { delta } => {
            let sign = if delta.is_above() { '+' } else { '-' };
            format!("{sign}{}", delta.magnitude())
        }
    }
}

fn badge_draws(signals: &CrossLevelSignals, active_level: Level) -> Vec<BadgeDraw> {
    let mut cells: Vec<Cell> = signals.cells().collect();
    cells.sort_by_key(|cell| (cell.x, cell.y));

    let mut draws = Vec::new();
    for cell in cells {
        for (kind, offset) in signals.badges_at(cell).iter().zip(BADGE_SLOT_OFFSETS) {
            let world = cell_to_world_layered(cell, active_level, Layer::CrossLevelSignal)
                + offset.extend(0.0);
            draws.push(BadgeDraw {
                world,
                tint: badge_tint(*kind),
                label: badge_label(*kind),
            });
        }
    }
    draws
}

fn label_world(tile_world: Vec3) -> Vec3 {
    Vec3::new(tile_world.x, tile_world.y, tile_world.z + LABEL_Z_LIFT)
}

/// Draw or refresh cross-level badges when signals or active level change.
pub fn draw_cross_level_signals(
    mut commands: Commands,
    signals: Res<CrossLevelSignals>,
    active: Res<ActiveLevel>,
    mut tiles: TileQuery,
    mut labels: LabelQuery,
) {
    if !signals.is_changed() && !active.is_changed() {
        return;
    }

    let active_level: Level = **active;
    let draws = badge_draws(&signals, active_level);

    draw_pool(
        tiles.iter_mut(),
        draws.clone(),
        |draw: BadgeDraw, (sprite, transform, _)| {
            sprite.color = draw.tint;
            transform.translation = draw.world;
        },
        |draw: BadgeDraw| spawn_tile(&mut commands, draw.tint, draw.world),
        |(_, _, visibility)| visibility,
    );

    draw_pool(
        labels.iter_mut(),
        draws,
        |draw: BadgeDraw, (label, transform, _)| {
            transform.translation = label_world(draw.world);
            **label = Text2d::new(draw.label);
        },
        |draw: BadgeDraw| {
            let world = label_world(draw.world);
            spawn_label(&mut commands, draw.label, world);
        },
        |(_, _, visibility)| visibility,
    );
}

fn spawn_tile(commands: &mut Commands, tint: Color, world: Vec3) {
    commands.spawn((
        CrossLevelBadgeTile,
        Sprite {
            color: tint,
            custom_size: Some(Vec2::splat(BADGE_SIZE_PX)),
            ..default()
        },
        Transform::from_translation(world),
        Visibility::Visible,
        RenderLayers::layer(WORLD_RENDER_LAYER),
    ));
}

fn spawn_label(commands: &mut Commands, text: String, world: Vec3) {
    commands.spawn((
        CrossLevelBadgeLabel,
        Text2d::new(text),
        TextFont {
            font_size: FontSize::Px(BADGE_LABEL_FONT_PX),
            ..default()
        },
        TextColor(BADGE_LABEL_COLOR),
        Anchor::CENTER,
        Transform::from_translation(world),
        Visibility::Visible,
        RenderLayers::layer(WORLD_RENDER_LAYER),
    ));
}
