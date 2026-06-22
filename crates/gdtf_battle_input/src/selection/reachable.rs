//! The reachable-range overlay POPULATE system (E7 · GTW-12i, C4): the input-crate half
//! of the move-range preview.
//!
//! The presenter owns the [`ReachableOverlay`] read-seam + the draw systems; this module
//! POPULATES that resource for the SELECTED ganger. It is the ONLY place
//! [`SelectedShooter`] feeds the reachable preview — keeping selection out of the
//! authoritative sim model (the `input → presenter → sim` direction; the presenter DEFINES
//! the resource, this input crate WRITES it, the [`HighlightRequest`](gdtf_battle_presenter::HighlightRequest)
//! precedent).
//!
//! It computes the reachable set the SAME way the move dispatch
//! ([`dispatch_move`](gdtf_battle_sim::acts::dispatch_move)) plans it: it builds the GTW-353
//! [`PlanningView`] from the sim's [`SquadVisibility`] with an occupant→[`FactionRelation`]
//! resolver closed over the selected ganger's faction, then calls
//! [`reachable_within`](gdtf_battle_sim::reachable_within) over the SAME grids + budget — so
//! the lit set EXACTLY matches the cells a commit will accept (the GTW-354 constrained
//! `dispatch_move` dependency / AC). It REUSES `reachable_within` — no re-implemented
//! reachability or cost.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_presenter::ReachableOverlay;
use gdtf_battle_sim::{
    CellLevel, CombatTuning, Faction, FactionRelation, OccupancyGrid, PlanningView, Position,
    SquadVisibility, Tu, VerticalLinkGraph, reachable_within,
};

use crate::selection::resources::SelectedShooter;

/// The faction relation of `occupant` relative to `mover_faction` — same gang is
/// [`FactionRelation::OwnSquad`], any other (or an occupant with no [`Faction`]) is
/// [`FactionRelation::Other`].
///
/// The [`PlanningView`] occupant→relation resolver, built per-rebuild from the selected
/// ganger's faction and the live `&`[`Faction`] query — a VERBATIM mirror of
/// `dispatch_move`'s own `relation_to` (the sim keeps that one private), so the overlay's
/// visibility gate is identical to the commit's. A non-ganger occupant maps to
/// [`FactionRelation::Other`] (the conservative classification: blocked only when its cell
/// is squad-VISIBLE).
fn relation_to(
    factions: &Query<&'static Faction>,
    mover_faction: Faction,
    occupant: Entity,
) -> FactionRelation {
    match factions.get(occupant) {
        Ok(faction) if *faction == mover_faction => FactionRelation::OwnSquad,
        _ => FactionRelation::Other,
    }
}

/// The grids the reachable flood reads, bundled as one [`SystemParam`] — the
/// [`OccupancyGrid`] + [`VerticalLinkGraph`] + [`SquadVisibility`] + [`CombatTuning`] the
/// SAME `dispatch_move` route gate reads. Framework plumbing (a borrow bundle), exempt from
/// no-bare-types; bundling them keeps [`populate_reachable_overlay`] under the
/// `too_many_arguments` / `too_many_lines` lints.
#[derive(SystemParam)]
pub struct RouteGrids<'w> {
    /// The coarse occupancy grid (terrain + occupants) the flood routes over.
    grid:   Res<'w, OccupancyGrid>,
    /// The vertical-link graph the flood stitches storeys through.
    links:  Res<'w, VerticalLinkGraph>,
    /// The squad fog the GTW-353 [`PlanningView`] gates routability by.
    squad:  Res<'w, SquadVisibility>,
    /// The combat tuning (the [`MoveCosts`](gdtf_battle_sim::MoveCosts) table + link cost).
    tuning: Res<'w, CombatTuning>,
}

/// `Update` ([`InputSystems::Gather`](crate::InputSystems)): POPULATE the presenter-owned
/// [`ReachableOverlay`] for the [`SelectedShooter`] — the cells it can reach within its
/// CURRENT [`Tu`] over the visibility-gated grid (C1 / C4).
///
/// When a ganger is selected and its `(`[`Position`]`, `[`Tu`]`, `[`Faction`]`)` resolves it
/// delegates to [`reachable_for`] (the SAME `PlanningView` + `reachable_within` construction
/// [`dispatch_move`](gdtf_battle_sim::acts::dispatch_move) uses) and writes the result;
/// otherwise it writes [`ReachableOverlay::cleared`] (the empty set). It writes only on a real
/// CHANGE (the `!=` guard) so an unchanged selection / budget / grid does not spuriously trip
/// `Changed<ReachableOverlay>` (C1 "no needless rebuild"). REUSES
/// [`reachable_within`](gdtf_battle_sim::reachable_within) — no re-implemented reachability,
/// selection NEVER pushed into the sim. Param-only (`bevy-traps.md` #7).
pub fn populate_reachable_overlay(
    selected: Res<SelectedShooter>,
    actors: Query<(&Position, &Tu, &Faction)>,
    factions: Query<&'static Faction>,
    grids: RouteGrids,
    mut overlay: ResMut<ReachableOverlay>,
) {
    let next = match (**selected).and_then(|entity| actors.get(entity).ok()) {
        Some((position, tu, &mover_faction)) => {
            reachable_for(**position, *tu, mover_faction, &factions, &grids)
        }
        // Nothing selected (or a missing component) → clear the overlay.
        None => ReachableOverlay::cleared(),
    };

    // Mutate only on a real change (C1 "no needless rebuild" / change-detection hygiene).
    if *overlay != next {
        *overlay = next;
    }
}

/// The reachable overlay for a ganger at `start` with `budget` TU and `mover_faction` — the
/// SAME visibility-gated [`PlanningView`] + [`reachable_within`] construction
/// [`dispatch_move`](gdtf_battle_sim::acts::dispatch_move) plans a route with, so the lit set
/// EXACTLY matches the cells a commit will accept (C1).
///
/// Builds the [`PlanningView`] from the squad fog + the per-call `relation_to` resolver (closed
/// over `mover_faction` + the live `&`[`Faction`] query), then floods from `start` within
/// `budget` over the [`RouteGrids`]. REUSES the sim flood — no re-implemented reachability /
/// cost.
fn reachable_for(
    start: CellLevel,
    budget: Tu,
    mover_faction: Faction,
    factions: &Query<&'static Faction>,
    grids: &RouteGrids,
) -> ReachableOverlay {
    // UNSEEN cells are non-routable; a visible enemy / own-squad ganger blocks; EXPLORED stays
    // routable — the GTW-353 gate, identical to the commit's.
    let planning = PlanningView::new(&grids.squad, |occupant| {
        relation_to(factions, mover_faction, occupant)
    });
    let cells = reachable_within(
        start,
        budget,
        &grids.grid,
        &grids.links,
        &grids.tuning,
        &planning,
    );
    ReachableOverlay::new(cells)
}
