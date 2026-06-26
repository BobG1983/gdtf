//! The automatic positional [`PeekOffset`] populator [`sync_peek_offsets`] and its
//! trigger gate [`peek_population_needed`] (GTW-406 §C).
//!
//! [`sync_peek_offsets`] re-derives every ganger's [`PeekOffset`] from the live
//! [`OccupancyGrid`] geometry (`corner_lean`) and writes it IN PLACE (direct
//! `&mut PeekOffset` mutation, mirroring `advance_walk`'s direct `*position` write) so
//! `Changed<PeekOffset>` fires in the SAME frame as the move/cover trigger that caused
//! it — consumed by that tick's
//! [`recompute_visibility`](crate::visibility::recompute_visibility), never the next.
//!
//! [`peek_population_needed`] gates the populator on the SAME signals that can change a
//! corner: a ganger moved/spawned (`Changed<`[`Position`]`>`) or a wall/cover was
//! destroyed ([`CoverDestroyed`] — the planar [`OccupancyGrid::is_blocked`] change). A
//! full unfiltered scan on either trigger (not a `Changed<Position>`-filtered query) is
//! deliberate: a cover destruction changes the corner status of nearby **stationary**
//! gangers a move-filter would miss. The diff-guard in the populator stops that scan
//! from tripping spurious `Changed<PeekOffset>` on the unmoved.

use bevy::prelude::{Changed, MessageReader, Query, Res};

use super::corner::corner_lean;
use crate::{
    ganger::Position, los::PeekOffset, occupancy::OccupancyGrid, occupancy_sync::CoverDestroyed,
};

/// Whether the positional peek offsets must be re-derived THIS tick — the populator's
/// run-condition (GTW-406 §C).
///
/// Returns `true` when EITHER signal that can change a corner occurred:
///
/// * a ganger **moved or spawned** (`Changed<`[`Position`]`>`) — its OWN corner may have
///   changed; or
/// * a wall/cover was **destroyed** ([`CoverDestroyed`] — the planar
///   [`OccupancyGrid::is_blocked`] change that can dissolve a *stationary* ganger's
///   corner).
///
/// Slabs do NOT affect the planar [`OccupancyGrid::is_blocked`] (they live in the
/// [`SurfaceGrid`](crate::surface::SurfaceGrid)), so `SlabDestroyed` is intentionally
/// NOT a trigger.
///
/// The [`CoverDestroyed`] reader is drained **unconditionally first** (`read().count()`)
/// and only THEN combined with the move check via `||`, so the [`MessageReader`] cursor
/// always advances every tick — matching
/// [`should_recompute_visibility`](crate::visibility::should_recompute_visibility)
/// (recompute.rs:85). A bare `moved || reader.read()…` would short-circuit when a ganger
/// also moved, leaving the cursor stuck on the old events and firing a spurious extra run
/// next tick. This run-condition reads the buffer with its OWN cursor, independent of
/// `should_recompute_visibility`'s drain (separate readers each see every message).
#[must_use]
pub fn peek_population_needed(
    moved: Query<(), Changed<Position>>,
    mut cover_destroyed: MessageReader<CoverDestroyed>,
) -> bool {
    // Drain the cover buffer FIRST, every tick, so the cursor advances regardless of the
    // move trigger (no spurious extra run from a stuck cursor — recompute.rs:85 pattern).
    let cover_changed = cover_destroyed.read().count() > 0;
    !moved.is_empty() || cover_changed
}

/// Re-derive every ganger's positional [`PeekOffset`] from the live
/// [`OccupancyGrid`] (GTW-406 §C — the maintenance system).
///
/// For each ganger, `corner_lean` computes the wall-peek lean from its current
/// [`Position`] + the grid; the result OVERWRITES its [`PeekOffset`] **wholesale**. The
/// component IS the memory — each run recomputes it from scratch, so there is no stale
/// incremental state to leak: a ganger that left a corner (moved away, cover destroyed,
/// open ground) is cleared to [`PeekOffset::default`] in the same sweep.
///
/// The write is a **direct `&mut PeekOffset` mutation** (not a deferred
/// `Commands::insert`), guarded by an exact-`Vec2` diff so it only touches a ganger whose
/// corner actually changed. Direct mutation makes `Changed<PeekOffset>` fire THIS frame —
/// in lockstep with `advance_walk`'s same-frame `Changed<Position>` — so this tick's
/// [`recompute_visibility`](crate::visibility::recompute_visibility) consumes it once,
/// with no spurious frame-N+1 recompute. The diff-guard is drift-free because the
/// populator only ever writes canonical constants (`0.0`, `±0.4`).
///
/// Every spawned ganger carries [`PeekOffset`] from the `bsn!` seed (GTW-406 §C spawn
/// prerequisite), so the `&mut PeekOffset` query silently skips nobody. The populator is
/// **stance-neutral** (it reads no [`Stance`](crate::ganger::Stance) — peek is a
/// horizontal lean `eye_anchor` composes with the stance-correct eye height) and
/// **faction-agnostic** (it populates enemies too, for a future AI / reaction-fire
/// consumer; an enemy's `Changed<PeekOffset>` never re-fires the player fog — the faction
/// filter in `should_recompute_visibility` covers that).
///
/// Param-only (`Res` / `Query`) — no `&mut World`, no `Commands` (`bevy-traps.md` #7).
pub fn sync_peek_offsets(
    grid: Res<OccupancyGrid>,
    mut gangers: Query<(&Position, &mut PeekOffset)>,
) {
    for (position, mut current) in &mut gangers {
        let next = corner_lean(**position, &grid);
        // Diff-guard: only a REAL geometry change trips deref_mut (and thus
        // Changed<PeekOffset>) — an unmoved ganger the full scan revisits stays untouched.
        if next != *current {
            *current = next;
        }
    }
}
