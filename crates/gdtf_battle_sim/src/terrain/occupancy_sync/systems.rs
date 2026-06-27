//! The three change-driven occupancy-maintenance systems — each edits the shared
//! [`OccupancyGrid`] IN PLACE off a SINGLE trigger (a `Changed<T>` filter or the
//! [`CoverDestroyed`] message reader), never a full-grid rebuild.

use bevy::prelude::{Changed, Commands, Entity, MessageReader, Or, Query, ResMut};

use crate::{
    clearance::silhouette_band,
    ganger::{LifeState, Position, Stance, StanceKind},
    occupancy::OccupancyGrid,
    occupancy_sync::{CoverDestroyed, GroundAccrued, PrevSlot, SlabDestroyed},
    surface::SurfaceGrid,
};

/// The read-set [`sync_moved_gangers`] queries per entity: its identity, current
/// slot ([`Position`]), optional [`Stance`] (the band source) and optional
/// [`PrevSlot`] (the old-slot memory). Aliased to keep the `Query` under the
/// `type_complexity` lint.
type MovedReads<'a> = (
    Entity,
    &'a Position,
    Option<&'a Stance>,
    Option<&'a PrevSlot>,
);

/// The change-detection filter [`sync_moved_gangers`] runs on: a ganger that **moved**
/// (`Changed<`[`Position`]`>`) **or re-posed** (`Changed<`[`Stance`]`>`). Aliased to
/// keep the `Query` under the `type_complexity` lint.
type MovedOrReposed = Or<(Changed<Position>, Changed<Stance>)>;

/// Maintain the occupancy grid for **moved or re-posed** gangers —
/// `Or<(Changed<`[`Position`]`>, Changed<`[`Stance`]`>)>`.
///
/// Reacts to Bevy change detection: for every entity whose [`Position`] **or**
/// [`Stance`] changed this frame, it clears the entity's OLD occupancy slot (read
/// from its [`PrevSlot`] bookkeeping, if any) and marks its NEW slot, then records
/// the new slot back into [`PrevSlot`]. The grid is edited **in place** via
/// [`OccupancyGrid::set_occupant`] / [`OccupancyGrid::set_occupant_band`] — it is
/// NEVER rebuilt (C6).
///
/// The OCCUPANT and its **silhouette band** are kept consistent: wherever the
/// occupant marker is set or cleared, its band is set or cleared in the same step
/// (the march only strikes a ganger when BOTH the occupant and its band are present
/// at a cell — `docs/combat/resolution.md` §2; a published occupant with no band
/// would be invisible to fire, GTW-304). The band is derived from the ganger's
/// [`Stance`] via [`silhouette_band`] (standing → HIGH, kneeling → MID, prone →
/// LOW); a ganger with no [`Stance`] component defaults to the structural
/// [`StanceKind::Standing`] silhouette.
///
/// **GTW-391 dual-cell stair occupancy.** For a non-prone ganger on a stair tile,
/// [`OccupancyGrid::register_stair_presence`] is called instead of the plain
/// `set_occupant` + `set_occupant_band` write. It atomically writes BOTH the lower
/// cell (stance band) and the upper `(cell, level+1)` (Low band), applying the
/// occupancy guard so it never stomps another entity's slot. The returned upper
/// [`CellLevel`](crate::metric::CellLevel) (or `None`) is recorded in [`PrevSlot`] for verbatim teardown.
///
/// Teardown is EXACT: the previous [`PrevSlot::upper`] is cleared
/// **unconditionally** (outside the `old != new` guard) so a prone re-pose in place
/// — which keeps the lower slot unchanged but must drop the upper — is handled
/// correctly (Test 7 in the GTW-391 stair suite).
///
/// Behavior per entity:
/// - **Old slot:** if the entity has a [`PrevSlot`] AND that slot's current
///   occupant is this entity, clear BOTH its occupant and its band
///   (`set_occupant(old, None)` + `set_occupant_band(old, None)`). The occupant
///   guard means a move never clobbers a slot another entity has since taken.
/// - **Old upper cell:** if the previous [`PrevSlot`] recorded an upper cell, clear
///   it **unconditionally** (via [`OccupancyGrid::clear_stair_upper`], which
///   applies its own ownership guard) — outside the `old != new` lower guard so
///   a prone re-pose in place correctly drops the upper presence.
/// - **New slot:** for a non-prone stair occupant, call
///   [`OccupancyGrid::register_stair_presence`] (dual write); otherwise, plain
///   `set_occupant` + `set_occupant_band`. Record the result in [`PrevSlot`].
/// - **First sync (initial placement):** a freshly-inserted [`Position`] reads as
///   `Changed` on the first tick (Bevy first-run semantics) with no prior
///   [`PrevSlot`] — there is no old slot to clear, so the system simply marks the
///   new slot(s) and records the [`PrevSlot`]. Initial placement is handled sanely.
/// - **Re-pose in place:** a [`Stance`] change with no move re-publishes the band
///   at the (unchanged) current slot, keeping the silhouette current.
///
/// `Commands` writes the [`PrevSlot`] bookkeeping; the occupant/band edits go
/// straight to the shared `ResMut<`[`OccupancyGrid`]`>`.
pub fn sync_moved_gangers(
    mut commands: Commands,
    mut grid: ResMut<OccupancyGrid>,
    moved: Query<MovedReads, MovedOrReposed>,
) {
    for (entity, position, stance, prev) in &moved {
        let new_lower = **position;
        let stance_kind = stance.map_or(StanceKind::Standing, |s| **s);
        let band = silhouette_band(stance_kind);

        // 1. CLEAR OLD SLOT + OLD UPPER (exact replay of what PrevSlot recorded).
        if let Some(prev) = prev {
            let old_lower = prev.slot();
            // Clear the lower slot only when the entity actually moved (guard against
            // stomping our own slot on a re-pose in place).
            if old_lower != new_lower && grid.occupant(&old_lower) == Some(entity) {
                grid.set_occupant(old_lower, None);
                grid.set_occupant_band(old_lower, None);
            }
            // Clear the old upper UNCONDITIONALLY — outside the `old != new` guard —
            // so a prone re-pose in place (lower stays fixed, stance goes prone) drops
            // the upper. The ownership guard inside clear_stair_upper keeps this safe.
            if let Some(old_upper) = prev.upper() {
                grid.clear_stair_upper(old_upper, entity);
            }
        }

        // 2. REGISTER NEW SLOT(S).
        //    Stair + non-prone → dual-cell (register_stair_presence writes both).
        //    Otherwise        → single-cell (set_occupant + set_occupant_band).
        let new_upper = if grid.is_stair_cell(&new_lower) && stance_kind != StanceKind::Prone {
            grid.register_stair_presence(new_lower, entity, band)
        } else {
            grid.set_occupant(new_lower, Some(entity));
            grid.set_occupant_band(new_lower, Some(band));
            None
        };

        // 3. RECORD EXACTLY WHAT WAS WRITTEN into PrevSlot.
        let prev_slot = match new_upper {
            Some(upper) => PrevSlot::with_upper(new_lower, upper),
            None => PrevSlot::new(new_lower),
        };
        commands.entity(entity).insert(prev_slot);
    }
}

/// Maintain the occupancy grid for **dead** gangers —
/// `Changed<`[`LifeState`]`>` filtered to ONLY [`LifeState::Dead`].
///
/// Reacts to Bevy change detection on [`LifeState`]: when an entity's life state
/// becomes [`LifeState::Dead`] (Wounds gone — a corpse, despawned soon), its
/// occupant marker is cleared from the slot it last synced to (read from
/// [`PrevSlot`]). An entity whose [`LifeState`] changed to anything OTHER than
/// `Dead` — [`LifeState::Alive`] OR [`LifeState::Downed`] — is left alone (no slot
/// edit). The grid is edited **in place**; it is never rebuilt.
///
/// **GTW-459 — a downed body HOLDS its cell, only Dead frees it.** A
/// [`LifeState::Downed`] ganger is NOT despawned (it keeps its `Position`, can be
/// stabilized / executed / recovered) and is a physical body still on the field, so
/// its cell stays BLOCKED for movement: the path planner
/// ([`is_open`](crate::pathfinder::PlanningView)) and the
/// [`advance_walk`](crate::move_acts) bump-stop both refuse a cell whose occupant
/// marker is set, so retaining the marker keeps an enemy from routing onto / through
/// a downed friendly (the live-play bug: a unit walked through a downed body because
/// its slot had been freed). Only a truly Dead ganger frees its cell; the previous
/// behavior — which freed the slot for Downed too — was the defect (GTW-459 C1). The
/// [`LifeState`] doc anticipated Downed and Dead differing for occupancy.
///
/// The slot cleared is the one in [`PrevSlot`] (the slot the move system last
/// synced this entity into); the occupant guard ensures only this entity's own
/// marker is cleared. The occupant's **silhouette band** is cleared in the same
/// step, keeping occupant and band consistent (GTW-304).
///
/// **GTW-459 fire-occlusion.** A Downed body retains BOTH its occupant marker AND
/// its silhouette band (this system never touches a non-Dead ganger), so it still
/// OCCLUDES fire — a body on the field blocks the march exactly as it blocks
/// movement (`docs/combat/resolution.md` §2: the march strikes / stops on a cell
/// carrying both an occupant and a band). Only the corpse (Dead → cleared here, then
/// despawned) leaves the line of fire.
///
/// **GTW-391 stair occupancy.** A ganger killed on a stair tile may have an upper
/// cell presence recorded in [`PrevSlot::upper`]. This system clears the upper cell
/// via [`OccupancyGrid::clear_stair_upper`] (ownership-guarded) so a dead
/// stair-occupant leaves no phantom hittable upper-cell Low band. A DOWNED
/// stair-occupant likewise RETAINS its upper-cell band (it is never reached here),
/// so it keeps blocking movement and occluding fire on both cells (GTW-459 C2).
pub fn sync_dead_gangers(
    mut grid: ResMut<OccupancyGrid>,
    downed: Query<(Entity, &LifeState, &PrevSlot), Changed<LifeState>>,
) {
    for (entity, life, prev) in &downed {
        // GTW-459: free the cell ONLY for a truly Dead ganger. A non-Dead ganger —
        // Alive (maintained by `sync_moved_gangers`, never freed here) OR Downed (a
        // body that HOLDS its cell) — retains both its occupant marker and its band.
        if !matches!(life, LifeState::Dead) {
            continue;
        }
        // Clear the lower slot (occupant + band), guarded by ownership.
        let slot = prev.slot();
        if grid.occupant(&slot) == Some(entity) {
            grid.set_occupant(slot, None);
            grid.set_occupant_band(slot, None);
        }
        // GTW-391: clear the upper stair cell if one was recorded (a ganger killed on
        // a stair must not leave a phantom hittable upper-cell Low band).
        if let Some(upper) = prev.upper() {
            grid.clear_stair_upper(upper, entity);
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

/// Maintain the surface grid for **destroyed slabs** — reads the buffered
/// [`SlabDestroyed`] message and folds each into the
/// [`SurfaceGrid`](crate::surface::SurfaceGrid) via
/// [`SurfaceGrid::destroy_slab`](crate::surface::SurfaceGrid::destroy_slab) (GTW-365;
/// the slab mirror of [`sync_destroyed_cover`]).
///
/// For every [`SlabDestroyed`] message buffered this frame, it calls
/// [`SurfaceGrid::destroy_slab`](crate::surface::SurfaceGrid::destroy_slab) with the
/// message's [`CellLevel`](crate::metric::CellLevel), setting that slab
/// [`SlabState::Destroyed`](crate::surface::SlabState) — **idempotent and permanent**
/// (a destroyed slab stays destroyed; re-receiving the message is a harmless no-op).
/// Once destroyed the slab stops blocking rounds AND the LOS march flies through it
/// (the march already honors `Destroyed` on BOTH the round and the sight path — it only
/// stops on `Present`), so this one grid edit reopens the sightline; the recompute
/// re-reveal is driven by the SAME `SlabDestroyed` message in
/// [`should_recompute_visibility`](crate::visibility::should_recompute_visibility). The
/// [`MessageReader`] is the ONLY trigger — no polling. The grid is edited **in place**.
///
/// `SlabDestroyed` is a buffered **message** (Bevy 0.18 — `bevy-traps.md` #4), hence
/// [`MessageReader`], not the pre-0.18 `EventReader`.
pub fn sync_destroyed_slab(
    mut surface: ResMut<SurfaceGrid>,
    mut destroyed: MessageReader<SlabDestroyed>,
) {
    for event in destroyed.read() {
        surface.destroy_slab(event.at);
    }
}

/// Maintain the surface grid for **ground damage** — reads the buffered
/// [`GroundAccrued`] message and folds each into the
/// [`SurfaceGrid`](crate::surface::SurfaceGrid)'s per-cell ground accumulator via
/// [`SurfaceGrid::accrue_ground_damage`](crate::surface::SurfaceGrid::accrue_ground_damage)
/// (GTW-366; the ground-accrual mirror of [`sync_destroyed_slab`]).
///
/// For every [`GroundAccrued`] message buffered this frame, it calls
/// [`SurfaceGrid::accrue_ground_damage`](crate::surface::SurfaceGrid::accrue_ground_damage)
/// with the message's [`Cell`](crate::metric::Cell) and
/// [`GroundDamage`](crate::surface::GroundDamage) amount, adding the round's damage onto
/// that cell's running total — **monotonic** (the accumulator only ever grows; there is
/// no lowering API) and **cosmetic** (the ground is damaged, never destroyed —
/// `docs/combat/resolution.md` §3.2). It mutates ONLY the ground accumulator: it touches
/// no slab state, no cover, no ganger — a future crater-FX reaction reads the total (the
/// crater render is out of scope, GTW-366 C5). The [`MessageReader`] is the ONLY trigger
/// — no polling. The grid is edited **in place**.
///
/// `GroundAccrued` is a buffered **message** (Bevy 0.18 — `bevy-traps.md` #4), hence
/// [`MessageReader`], not the pre-0.18 `EventReader`.
pub fn sync_accrued_ground(
    mut surface: ResMut<SurfaceGrid>,
    mut accrued: MessageReader<GroundAccrued>,
) {
    for event in accrued.read() {
        surface.accrue_ground_damage(event.cell, event.amount);
    }
}
