//! The three change-driven occupancy-maintenance systems — each edits the shared
//! [`OccupancyGrid`] IN PLACE off a SINGLE trigger (a `Changed<T>` filter or the
//! [`CoverDestroyed`] message reader), never a full-grid rebuild.

use bevy::prelude::{Changed, Commands, Entity, MessageReader, Query, ResMut};

use crate::{
    ganger::{LifeState, Position},
    occupancy::OccupancyGrid,
    occupancy_sync::{CoverDestroyed, PrevSlot},
};

/// Maintain the occupancy grid for **moved** gangers — `Changed<`[`Position`]`>`.
///
/// Reacts to Bevy change detection: for every entity whose [`Position`] changed
/// this frame, it clears the entity's OLD occupancy slot (read from its
/// [`PrevSlot`] bookkeeping, if any) and marks its NEW slot, then records the new
/// slot back into [`PrevSlot`]. The grid is edited **in place** via
/// [`OccupancyGrid::set_occupant`] — it is NEVER rebuilt (C6).
///
/// Behavior per entity:
/// - **Old slot:** if the entity has a [`PrevSlot`] AND that slot's current
///   occupant is this entity, clear it (`set_occupant(old, None)`). The occupant
///   guard means a move never clobbers a slot another entity has since taken.
/// - **New slot:** mark `set_occupant(new, Some(entity))` and write
///   `PrevSlot(new)`.
/// - **First sync (initial placement):** a freshly-inserted [`Position`] reads as
///   `Changed` on the first tick (Bevy first-run semantics) with no prior
///   [`PrevSlot`] — there is no old slot to clear, so the system simply marks the
///   new slot and records the [`PrevSlot`]. Initial placement is handled sanely.
///
/// `Commands` writes the [`PrevSlot`] bookkeeping; the occupant edits go straight
/// to the shared `ResMut<`[`OccupancyGrid`]`>`.
pub fn sync_moved_gangers(
    mut commands: Commands,
    mut grid: ResMut<OccupancyGrid>,
    moved: Query<(Entity, &Position, Option<&PrevSlot>), Changed<Position>>,
) {
    for (entity, position, prev) in &moved {
        let new_slot = **position;
        // Clear the OLD slot, but only if we still own it — a move must not stomp
        // a slot another entity has taken since (C3: clear OLD, mark NEW).
        if let Some(prev) = prev {
            let old_slot = prev.slot();
            if old_slot != new_slot && grid.occupant(&old_slot) == Some(entity) {
                grid.set_occupant(old_slot, None);
            }
        }
        // Mark the NEW slot and remember it for the next move.
        grid.set_occupant(new_slot, Some(entity));
        commands.entity(entity).insert(PrevSlot::new(new_slot));
    }
}

/// Maintain the occupancy grid for **downed / dead** gangers —
/// `Changed<`[`LifeState`]`>` filtered to non-[`LifeState::Alive`].
///
/// Reacts to Bevy change detection on [`LifeState`]: when an entity's life state
/// changes to [`LifeState::Downed`] or [`LifeState::Dead`], its occupant marker is
/// cleared from the slot it last synced to (read from [`PrevSlot`]). An entity
/// whose [`LifeState`] changed but is still [`LifeState::Alive`] is left alone (no
/// slot edit) — only going OUT frees the cell (C4). The grid is edited **in
/// place**; it is never rebuilt.
///
/// The slot cleared is the one in [`PrevSlot`] (the slot the move system last
/// synced this entity into); the occupant guard ensures only this entity's own
/// marker is cleared.
pub fn sync_dead_gangers(
    mut grid: ResMut<OccupancyGrid>,
    downed: Query<(Entity, &LifeState, &PrevSlot), Changed<LifeState>>,
) {
    for (entity, life, prev) in &downed {
        if matches!(life, LifeState::Alive) {
            continue;
        }
        let slot = prev.slot();
        if grid.occupant(&slot) == Some(entity) {
            grid.set_occupant(slot, None);
        }
    }
}

/// Maintain the occupancy grid for **destroyed cover** — reads the buffered
/// [`CoverDestroyed`] message and folds each into the grid's destroyed-cover set.
///
/// For every [`CoverDestroyed`] message buffered this frame, it calls
/// [`OccupancyGrid::mark_cover_destroyed`] with the message's
/// [`CellLevel`](crate::metric::CellLevel), adding the cell to the grid's
/// append-only destroyed-cover set (so a smashed piece stops blocking and can never
/// resurrect). The [`MessageReader`] is the ONLY trigger — no polling, no full-grid
/// scan (C6). The grid is edited **in place**.
///
/// `CoverDestroyed` is a buffered **message** (Bevy 0.18 — `bevy-traps.md` #4),
/// hence [`MessageReader`], not the pre-0.18 `EventReader`.
pub fn sync_destroyed_cover(
    mut grid: ResMut<OccupancyGrid>,
    mut destroyed: MessageReader<CoverDestroyed>,
) {
    for event in destroyed.read() {
        grid.mark_cover_destroyed(event.at);
    }
}
