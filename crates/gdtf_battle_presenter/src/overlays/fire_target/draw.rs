use bevy::{camera::visibility::RenderLayers, prelude::*, text::TextColor};
use gdtf_battle_sim::prelude::{CellLevel, Level, Tu};

use crate::{
    ActiveLevel, CELL_PX, Layer, WORLD_RENDER_LAYER, cell_to_world_layered,
    overlays::pool::draw_pool,
};

#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FireTargetHighlight {
            target: Option<(CellLevel, Tu)>,
}

impl FireTargetHighlight {
            #[must_use]
    pub const fn new(cell: CellLevel, cost: Tu) -> Self {
        Self {
            target: Some((cell, cost)),
        }
    }

            #[must_use]
    pub const fn cleared() -> Self {
        Self { target: None }
    }

        #[must_use]
    pub const fn cell(&self) -> Option<CellLevel> {
        match self.target {
            Some((cell, _)) => Some(cell),
            None => None,
        }
    }

            #[must_use]
    pub const fn cost(&self) -> Option<Tu> {
        match self.target {
            Some((_, cost)) => Some(cost),
            None => None,
        }
    }

        #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.target.is_none()
    }
}

#[derive(Component, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct FireTargetTile;

#[derive(Component, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct FireTargetLabel;

type TileQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Transform, &'static mut Visibility),
    (With<FireTargetTile>, Without<FireTargetLabel>),
>;

type LabelQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Text2d,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    (With<FireTargetLabel>, Without<FireTargetTile>),
>;

/// (the `CELL_PX`-class const carve-out, the [`PREVIEW_TINT`](crate::path_preview) precedent).
const FIRE_TARGET_TINT: Color = Color::srgba(1.0, 0.15, 0.1, 0.5);

const COST_LABEL_COLOR: Color = Color::srgb(0.95, 1.0, 0.95);

const COST_LABEL_FONT_PX: f32 = 9.0;

const COST_LABEL_LIFT_PX: f32 = CELL_PX * 0.55;

pub fn draw_fire_target(
    mut commands: Commands,
    highlight: Res<FireTargetHighlight>,
    active: Res<ActiveLevel>,
    mut tile: TileQuery,
    mut label: LabelQuery,
) {
    let active_level: Level = **active;
    let active_z = i32::from(*active_level);
    let drawable = highlight
        .cell()
        .filter(|cell| cell.z == active_z)
        .zip(highlight.cost());

    draw_tile(&mut commands, drawable, active_level, &mut tile);
    draw_cost_label(&mut commands, drawable, active_level, &mut label);
}

fn draw_tile(
    commands: &mut Commands,
    drawable: Option<(CellLevel, Tu)>,
    active_level: Level,
    tile_query: &mut TileQuery,
) {
    let draws = drawable
        .map(|(cell, _cost)| cell_to_world_layered(cell.cell(), active_level, Layer::FireTarget));
    draw_pool(
        tile_query.iter_mut(),
        draws,
        |world, (transform, _)| transform.translation = world,
        |world| spawn_tile(commands, world),
        |(_, visibility)| visibility,
    );
}

fn draw_cost_label(
    commands: &mut Commands,
    drawable: Option<(CellLevel, Tu)>,
    active_level: Level,
    label_query: &mut LabelQuery,
) {
    let world_at = |cell: CellLevel| {
        let mut world = cell_to_world_layered(cell.cell(), active_level, Layer::FireTarget);
        world.y += COST_LABEL_LIFT_PX;
        world
    };
    draw_pool(
        label_query.iter_mut(),
        drawable,
        |(cell, cost), (label, transform, _)| {
            let label: &mut Text2d = label;
            **label = cost_label_text(cost);
            transform.translation = world_at(cell);
        },
        |(cell, cost)| spawn_cost_label(commands, cost_label_text(cost), world_at(cell)),
        |(_, _, visibility)| visibility,
    );
}

pub(super) fn cost_label_text(cost: Tu) -> String {
    format!("{} TU", *cost)
}

fn spawn_tile(commands: &mut Commands, world: Vec3) {
    commands.spawn((
        FireTargetTile,
        Sprite {
            color: FIRE_TARGET_TINT,
            custom_size: Some(Vec2::splat(CELL_PX)),
            ..default()
        },
        Transform::from_translation(world),
        Visibility::Visible,
        RenderLayers::layer(WORLD_RENDER_LAYER),
    ));
}

fn spawn_cost_label(commands: &mut Commands, text: String, world: Vec3) {
    commands.spawn((
        FireTargetLabel,
        Text2d::new(text),
        TextFont {
            font_size: bevy::text::FontSize::Px(COST_LABEL_FONT_PX),
            ..default()
        },
        TextColor(COST_LABEL_COLOR),
        bevy::sprite::Anchor::BOTTOM_CENTER,
        Transform::from_translation(world),
        Visibility::Visible,
        RenderLayers::layer(WORLD_RENDER_LAYER),
    ));
}
