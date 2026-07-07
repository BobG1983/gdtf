//! The drawn-storey band: the RANGE form of the shared storey-treatment classification
//! ([`drawn_band`], derived from [`storey_treatment`] — GTW-594 C1) plus its predicates.

use std::ops::RangeInclusive;

use gdtf_battle_sim::{
    metric::MAX_LEVELS,
    prelude::{CellLevel, Level},
};

use super::{
    active_level::ActiveLevel,
    treatment::{StoreyTreatment, StoreyViewMode, storey_treatment},
};

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

/// The inclusive band of storeys the terrain draw renders (GTW-519 / GTW-521 / GTW-594),
/// given the current [`ActiveLevel`] and the composed [`StoreyViewMode`].
///
/// GTW-594 (C1): the band is DERIVED from the ONE shared storey-treatment classifier
/// ([`storey_treatment`]) — it is the contiguous span of storeys that classify non-
/// [`Hidden`](StoreyTreatment::Hidden), so this range and the per-storey classifications
/// can never drift. The three mode shapes it derives:
///
/// - `DownToActive` (Isolate off, the default) → `0..=active`: the whole stack at/below
///   the active view level, culling strictly above — EXACTLY the GTW-519/520 behaviour.
/// - `FullView` (Isolate off) → `0..=MAX_LEVELS - 1`: the WHOLE storey stack regardless of
///   the active level (the UFO full-stack view); empty upper cells still emit nothing
///   (peek-through), so it stays cheap.
/// - Isolate ON (WINS over both — the GTW-594 C3 precedence) → `active - onion ..= active`
///   (saturating at the ground): the band FLOOR is the active storey minus its onion
///   depth, no longer always the ground plane.
///
/// The painter's-algorithm occlusion falls out of the existing per-storey Z ([`z_for`], via
/// [`cell_to_world`](crate::cell_to_world)) either way — a higher storey's tile carries a
/// strictly greater z, so it
/// draws in front, with NO new Z math.
///
/// The SINGLE readable range form of "which storeys are drawn", shared by the draw loop
/// (C1) AND the two destroyed-swap reactions (C6,
/// [`swap_destroyed_cover`](super::swaps::swap_destroyed_cover) /
/// [`swap_destroyed_slab`](super::swaps::swap_destroyed_slab)) — the GTW-520 ganger
/// visibility filter reads the same classifier through
/// [`ActiveLevel::draws_storey`](super::active_level::ActiveLevel) — so they can never
/// drift.
///
/// [`z_for`]: crate::cell_to_world
pub(super) fn drawn_band(active: ActiveLevel, mode: StoreyViewMode) -> RangeInclusive<Level> {
    // Derive the span from the classifier: the active storey always classifies Active
    // (the exactly-one-Active law), so the fold's seed is the active index; every other
    // non-Hidden storey widens the floor/ceiling. All three modes classify a CONTIGUOUS
    // drawn set, so the min/max span IS the drawn set.
    let mut floor = **active;
    let mut ceiling = **active;
    for storey in 0..MAX_LEVELS {
        if storey_treatment(Level::new(storey), active, mode) != StoreyTreatment::Hidden {
            floor = floor.min(storey);
            ceiling = ceiling.max(storey);
        }
    }
    Level::new(floor)..=Level::new(ceiling)
}
