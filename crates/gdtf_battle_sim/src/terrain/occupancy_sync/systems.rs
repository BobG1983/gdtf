//! Systems that project ganger and terrain changes onto occupancy and surface grids.

use bevy::prelude::{Changed, Commands, Entity, MessageReader, Or, Query, ResMut};

use crate::{
    clearance::silhouette_band,
    ganger::{LifeState, Position, Stance, StanceKind},
    metric::CellLevel,
    occupancy::{BodyOcclusion, OccupancyGrid},
    occupancy_sync::{GroundAccrued, PrevSlot},
    surface::SurfaceGrid,
};

type MovedReads<'a> = (
    Entity,
    &'a Position,
    Option<&'a Stance>,
    Option<&'a LifeState>,
    Option<&'a PrevSlot>,
);

type MovedOrReposed = Or<(Changed<Position>, Changed<Stance>)>;

// At `slot`, prone: a corpse goes in the body channel, a downed ganger keeps the occupant slot.
fn publish_body(grid: &mut OccupancyGrid, entity: Entity, life: LifeState, slot: CellLevel) {
    let band = silhouette_band(StanceKind::Prone);
    if matches!(life, LifeState::Dead) {
        if grid.occupant(&slot) == Some(entity) {
            grid.set_occupant(slot, None);
            grid.set_occupant_band(slot, None);
        }
        grid.set_body(slot, Some(BodyOcclusion::new(entity, band)));
    } else {
        grid.set_occupant(slot, Some(entity));
        grid.set_occupant_band(slot, Some(band));
    }
}

/// Update grid occupancy when a ganger moves or changes stance.
pub fn sync_moved_gangers(
    mut commands: Commands,
    mut grid: ResMut<OccupancyGrid>,
    moved: Query<MovedReads, MovedOrReposed>,
) {
    for (entity, position, stance, life, prev) in &moved {
        let new_lower = **position;
        let stance_kind = stance.map_or(StanceKind::Standing, |s| **s);
        let band = silhouette_band(stance_kind);
        let life = life.copied().unwrap_or_default();

        if let Some(prev) = prev {
            let old_lower = prev.slot();
            if old_lower != new_lower && grid.occupant(&old_lower) == Some(entity) {
                grid.set_occupant(old_lower, None);
                grid.set_occupant_band(old_lower, None);
            }
            if let Some(old_upper) = prev.upper() {
                grid.clear_stair_upper(old_upper, entity);
            }
        }

        let new_upper = if !*life.is_active() {
            publish_body(&mut grid, entity, life, new_lower);
            None
        } else if *grid.is_stair_cell(&new_lower) && stance_kind != StanceKind::Prone {
            grid.register_stair_presence(new_lower, entity, band)
        } else {
            grid.set_occupant(new_lower, Some(entity));
            grid.set_occupant_band(new_lower, Some(band));
            None
        };

        let prev_slot = match new_upper {
            Some(upper) => PrevSlot::with_upper(new_lower, upper),
            None => PrevSlot::new(new_lower),
        };
        commands.entity(entity).insert(prev_slot);
    }
}

/// Drop a ganger to the floor when it can no longer act.
pub fn sync_inactive_gangers(
    mut grid: ResMut<OccupancyGrid>,
    downed: Query<(Entity, &LifeState, &PrevSlot), Changed<LifeState>>,
) {
    for (entity, life, prev) in &downed {
        if *life.is_active() {
            continue;
        }
        publish_body(&mut grid, entity, *life, prev.slot());
        if let Some(upper) = prev.upper() {
            grid.clear_stair_upper(upper, entity);
        }
    }
}

/// Accrue ground damage on the surface grid.
pub fn sync_accrued_ground(
    mut surface: ResMut<SurfaceGrid>,
    mut accrued: MessageReader<GroundAccrued>,
) {
    for event in accrued.read() {
        surface.accrue_ground_damage(event.cell, event.amount);
    }
}
