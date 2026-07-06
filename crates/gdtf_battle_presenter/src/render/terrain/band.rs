//! The drawn-storey band: the ONE shared which-storeys-are-drawn definition
//! ([`drawn_band`]) plus its predicates.

use std::ops::RangeInclusive;

use gdtf_battle_sim::{
    metric::MAX_LEVELS,
    prelude::{CellLevel, Level},
};

use super::active_level::{ActiveLevel, ViewMode};

/// Whether a `(cell, level)` key's storey lies within the drawn [`drawn_band`] (GTW-519 C6).
///
/// The shared predicate the two destroyed-swap reactions
/// ([`swap_destroyed_cover`](super::swaps::swap_destroyed_cover) /
/// [`swap_destroyed_slab`](super::swaps::swap_destroyed_slab)) gate on so they act on any
/// DRAWN storey (`0..=active`) and ignore
/// one strictly above the active view level. A [`CellLevel`]'s `z` is the `i32` storey index
/// ([`CellLevel`] wraps `IVec3`); the band's inclusive [`Level`] bounds read through
/// [`Level`]'s `Deref<Target = u8>` and compare against it. A negative or over-`u8` `z` (a
/// can't-happen malformed key) simply fails the bound rather than panicking.
pub(super) fn cell_level_in_band(at: CellLevel, band: &RangeInclusive<Level>) -> bool {
    let start = i32::from(**band.start());
    let end = i32::from(**band.end());
    (start..=end).contains(&at.z)
}

/// Iterate the [`drawn_band`] as concrete [`Level`]s (BOTTOM-UP, `0..=active` inclusive).
///
/// A small adapter over the [`RangeInclusive<Level>`] the shared [`drawn_band`] returns:
/// [`Level`] wraps a `u8` but is not itself `Step` (no `Iterator` for the range), so this
/// walks the inclusive `u8` storey indices and re-wraps each through [`Level::new`], keeping
/// the loop bottom-up so painter's-Z occlusion holds by draw order + the per-storey z.
pub(super) fn level_band(band: RangeInclusive<Level>) -> impl Iterator<Item = Level> {
    (**band.start()..=**band.end()).map(Level::new)
}

/// The inclusive band of storeys the terrain draw renders (GTW-519 / GTW-521), given the
/// current [`ActiveLevel`] and [`ViewMode`].
///
/// The UFO:EU / `OpenXcom` multi-level display: the band FLOOR is always the ground floor
/// (level 0 — the whole stack at/below the ceiling, not a windowed `[ceiling-N..=ceiling]`,
/// the GTW-519 logged fork (a)). The CEILING is the ONE thing the [`ViewMode`] chooses
/// (GTW-521):
///
/// - [`ViewMode::DownToActive`] (the default) → `0..=active`: draw up to and including the
///   active view level and CULL everything strictly above it — EXACTLY the GTW-519/520
///   behaviour, unchanged.
/// - [`ViewMode::FullView`] → `0..=MAX_LEVELS - 1`: draw the WHOLE storey stack regardless of
///   the active level (the UFO full-stack view). The upper roofs / floors re-appear and hide
///   the storeys / units beneath by per-storey Z; empty upper cells still emit nothing
///   (peek-through), so it stays cheap.
///
/// The painter's-algorithm occlusion falls out of the existing per-storey Z ([`z_for`], via
/// [`cell_to_world`](crate::cell_to_world)) either way — a higher storey's tile carries a
/// strictly greater z, so it
/// draws in front, with NO new Z math.
///
/// The SINGLE readable definition of "which storeys are drawn", shared by the draw loop
/// (C1) AND the two destroyed-swap reactions (C6,
/// [`swap_destroyed_cover`](super::swaps::swap_destroyed_cover) /
/// [`swap_destroyed_slab`](super::swaps::swap_destroyed_slab)) — and, through
/// [`ActiveLevel::draws_storey`](super::active_level::ActiveLevel), the GTW-520 ganger
/// visibility filter — so they can never drift. GTW-521's full-view toggle widens the band's
/// CEILING here in this ONE helper WITHOUT re-touching the loop body or the swap predicates.
///
/// [`z_for`]: crate::cell_to_world
pub(super) fn drawn_band(active: ActiveLevel, view: ViewMode) -> RangeInclusive<Level> {
    // The band floor is always the ground plane; only the ceiling depends on the view mode.
    let ceiling = match view {
        // DEFAULT (GTW-519/520): cull above the active view level.
        ViewMode::DownToActive => *active,
        // FULL VIEW (GTW-521): the top valid storey (`MAX_LEVELS - 1` = 7). `saturating_sub`
        // guards the impossible `MAX_LEVELS == 0`.
        ViewMode::FullView => Level::new(MAX_LEVELS.saturating_sub(1)),
    };
    Level::new(0)..=ceiling
}
