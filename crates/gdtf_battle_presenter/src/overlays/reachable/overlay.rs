//! Reachable-range cell tint, render-only. Compiles in every build; the draw system
//! registers in debug builds only.

use bevy::{camera::visibility::RenderLayers, prelude::*};
use gdtf_battle_sim::prelude::{CellLevel, Level, Tu};

use crate::{
    ActiveLevel, CELL_PX, Layer, WORLD_RENDER_LAYER, cell_to_world_layered,
    overlays::pool::draw_pool,
};

/// Whether the reachable-range overlay draws. The `view.toggle_reachable_overlay`
/// command flips it at runtime.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq, Deref)]
pub struct ReachableOverlayEnabled(bool);

impl ReachableOverlayEnabled {
    /// Build from an explicit flag.
    #[must_use]
    pub const fn new(enabled: bool) -> Self {
        Self(enabled)
    }

    /// The same flag the other way round.
    #[must_use]
    pub const fn flipped(self) -> Self {
        Self(!self.0)
    }
}

/// Cells currently in the selected ganger's reach, with TU costs.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct ReachableCells(Vec<ReachableCell>);

impl ReachableCells {
    /// Build from cell/cost pairs.
    #[must_use]
    pub fn new(cells: impl IntoIterator<Item = (CellLevel, Tu)>) -> Self {
        Self(
            cells
                .into_iter()
                .map(|(cell, cost)| ReachableCell { cell, cost })
                .collect(),
        )
    }

    /// Empty reach set.
    #[must_use]
    pub const fn cleared() -> Self {
        Self(Vec::new())
    }

    /// Iterate reachable cells and their costs.
    pub fn cells(&self) -> impl Iterator<Item = (CellLevel, Tu)> + '_ {
        self.0.iter().map(|r| (r.cell, r.cost))
    }

    /// Whether no cells are reachable.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReachableCell {
    cell: CellLevel,
    cost: Tu,
}

/// Marker on a reachable-range tint sprite.
#[derive(Component, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ReachableCellSprite;

const REACHABLE_TINT: Color = Color::srgba(0.2, 0.9, 0.4, 0.4);

type ReachableSpriteQuery<'w, 's> =
    Query<'w, 's, (&'static mut Transform, &'static mut Visibility), With<ReachableCellSprite>>;

pub(super) fn reachable_draws(reachable: &ReachableCells, active_level: Level) -> Vec<CellLevel> {
    let active_z = i32::from(*active_level);
    reachable
        .cells()
        .map(|(cell, _cost)| cell)
        .filter(|cell| cell.z == active_z)
        .collect()
}

/// Draw reachable-range tints for the active storey.
pub fn draw_reachable_overlay(
    mut commands: Commands,
    reachable: Res<ReachableCells>,
    active: Res<ActiveLevel>,
    mut sprites: ReachableSpriteQuery,
) {
    let active_level: Level = **active;
    let draws = reachable_draws(&reachable, active_level);

    let world_at =
        |cell: CellLevel| cell_to_world_layered(cell.cell(), active_level, Layer::ReachableRange);
    draw_pool(
        sprites.iter_mut(),
        draws,
        |cell, (transform, _)| transform.translation = world_at(cell),
        |cell| spawn_reachable_sprite(&mut commands, world_at(cell)),
        |(_, visibility)| visibility,
    );
}

fn spawn_reachable_sprite(commands: &mut Commands, world: Vec3) {
    commands.spawn((
        ReachableCellSprite,
        Sprite {
            color: REACHABLE_TINT,
            custom_size: Some(Vec2::splat(CELL_PX)),
            ..default()
        },
        Transform::from_translation(world),
        Visibility::Visible,
        RenderLayers::layer(WORLD_RENDER_LAYER),
    ));
}
