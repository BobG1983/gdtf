//! Path step tiles and destination TU label.

use bevy::{camera::visibility::RenderLayers, prelude::*, text::TextColor};
use gdtf_battle_sim::{
    prelude::{CellLevel, Level, Tu},
    visibility::SquadVisibility,
};

use super::{resolve::preview_draws, seam::PathPreview};
use crate::{
    ActiveLevel, CELL_PX, Layer, WORLD_RENDER_LAYER, cell_to_world_layered,
    overlays::pool::draw_pool,
};

/// Marker on a path-step tint sprite.
#[derive(Component, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct PathStepSprite;

type StepQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Transform,
        &'static mut Sprite,
        &'static mut Visibility,
    ),
    (With<PathStepSprite>, Without<PathTargetLabel>),
>;

type LabelQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Text2d,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    (With<PathTargetLabel>, Without<PathStepSprite>),
>;

/// Marker on the path destination cost label.
#[derive(Component, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct PathTargetLabel;

pub(super) const LABEL_COLOR: Color = Color::srgb(0.95, 1.0, 0.95);

const LABEL_FONT_PX: f32 = 9.0;

const LABEL_LIFT_PX: f32 = CELL_PX * 0.55;

/// Draw path steps (visibility-tinted) and the destination cost label.
pub fn draw_path_preview(
    mut commands: Commands,
    preview: Res<PathPreview>,
    active: Res<ActiveLevel>,
    squad: Res<SquadVisibility>,
    mut steps: StepQuery,
    mut label: LabelQuery,
) {
    let active_level: Level = **active;
    let draws = preview_draws(&preview, active_level, &squad);

    let world_at =
        |cell: CellLevel| cell_to_world_layered(cell.cell(), active_level, Layer::PathPreview);
    draw_pool(
        steps.iter_mut(),
        draws,
        |draw, (transform, sprite, _)| {
            transform.translation = world_at(draw.cell);
            sprite.color = draw.tint;
        },
        |draw| spawn_step(&mut commands, world_at(draw.cell), draw.tint),
        |(_, _, visibility)| visibility,
    );

    draw_target_label(&mut commands, &preview, active_level, &mut label);
}

fn draw_target_label(
    commands: &mut Commands,
    preview: &PathPreview,
    active_level: Level,
    label_query: &mut LabelQuery,
) {
    let active_z = i32::from(*active_level);
    let target = preview
        .cells()
        .last()
        .copied()
        .filter(|cell| cell.z == active_z);

    let world_at = |cell: CellLevel| {
        let mut world = cell_to_world_layered(cell.cell(), active_level, Layer::PathPreview);
        world.y += LABEL_LIFT_PX;
        world
    };
    draw_pool(
        label_query.iter_mut(),
        target,
        |cell, (label, transform, _)| {
            let label: &mut Text2d = label;
            **label = label_text(preview.cost());
            transform.translation = world_at(cell);
        },
        |cell| spawn_target_label(commands, label_text(preview.cost()), world_at(cell)),
        |(_, _, visibility)| visibility,
    );
}

pub(super) fn label_text(cost: Tu) -> String {
    format!("{} TU", *cost)
}

fn spawn_target_label(commands: &mut Commands, text: String, world: Vec3) {
    commands.spawn((
        PathTargetLabel,
        Text2d::new(text),
        TextFont {
            font_size: bevy::text::FontSize::Px(LABEL_FONT_PX),
            ..default()
        },
        TextColor(LABEL_COLOR),
        bevy::sprite::Anchor::BOTTOM_CENTER,
        Transform::from_translation(world),
        Visibility::Visible,
        RenderLayers::layer(WORLD_RENDER_LAYER),
    ));
}

fn spawn_step(commands: &mut Commands, world: Vec3, tint: Color) {
    commands.spawn((
        PathStepSprite,
        Sprite {
            color: tint,
            custom_size: Some(Vec2::splat(CELL_PX)),
            ..default()
        },
        Transform::from_translation(world),
        Visibility::Visible,
        RenderLayers::layer(WORLD_RENDER_LAYER),
    ));
}
