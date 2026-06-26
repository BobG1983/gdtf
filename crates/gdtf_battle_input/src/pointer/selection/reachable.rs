//! The reachable-range overlay seam + POPULATE system (GTW-387 C3): the input-crate half
//! of the reachable-range draw.
//!
//! The presenter owns the [`ReachableCells`] read-seam + the draw system; this module
//! POPULATES the presenter resource for the SELECTED ganger using
//! [`reachable_within`](gdtf_battle_sim::reachable_within) — the SAME sim API the
//! dispatch and test harnesses use. It is the ONLY place [`SelectedShooter`] feeds the
//! reachable-range overlay — keeping selection out of the authoritative sim model (the
//! `input → presenter → sim` direction; the presenter DEFINES the resource, this input
//! crate WRITES it, the [`PathPreview`](gdtf_battle_presenter::PathPreview) precedent).
//!
//! # The GTW-387 B fix dependency
//!
//! `reachable_within` now traverses vertical links even when their far endpoint is UNSEEN
//! (the fog-routing relaxation, GTW-387 B). So the reachable set it returns can include
//! UNSEEN link-head cells on other storeys — those are valid destinations that a ganger
//! can plan to via a known stair/ladder. The draw system hard-cuts to the active storey,
//! so after a `PageUp` to L1 those L1 cells render.
//!
//! # `OQ-4` `NoOp` — link tiles stay non-clickable
//!
//! The overlay adds NO click affordance to link tiles. `decision.rs` still returns `NoOp`
//! for a link tile. Ascent stays `PageUp` → click an L1 floor tile (a platform cell like
//! `(3,2,L1)` added by GTW-387 A). The overlay is purely a RENDER of where you COULD move.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_presenter::ReachableCells;
use gdtf_battle_sim::{
    CellLevel, CombatTuning, Faction, FactionRelation, FloorCostGrid, InflictedInjuries,
    MovementCostFactor, OccupancyGrid, PlanningView, Position, SquadVisibility, Tu,
    VerticalLinkGraph, reachable_within,
};

use crate::selection::resources::SelectedShooter;

/// `Update` ([`InputSystems::Gather`](crate::InputSystems)): POPULATE the
/// presenter-owned [`ReachableCells`] for the [`SelectedShooter`] — the
/// [`reachable_within`](gdtf_battle_sim::reachable_within) flood over the
/// visibility-gated grid within the selected ganger's remaining TU (C3).
///
/// When a ganger is selected and its `(`[`Position`]`,` [`Tu`]`,` [`Faction`]`)` resolves,
/// it builds the [`PlanningView`] from the squad fog with the same occupant→faction
/// resolver pattern as [`populate_path_preview`](super::path_preview::populate_path_preview),
/// calls [`reachable_within`] over the SAME grids, wraps the result in [`ReachableCells`],
/// and writes it to the presenter resource. When no ganger is selected (or a component is
/// missing) it clears the resource. Writes only on a real CHANGE (the `!=` guard) to keep
/// `Changed<ReachableCells>` honest. Param-only (`bevy-traps.md` #7).
///
/// The reachable set spans ALL storeys the ganger can reach (including UNSEEN link-head
/// cells via the GTW-387 B relaxation); the presenter draw system hard-cuts to the active
/// storey and renders only the active level's members.
pub fn populate_reachable_overlay(
    selected: Res<SelectedShooter>,
    actors: Query<(&Position, &Tu, &Faction, Option<&InflictedInjuries>)>,
    factions: Query<&'static Faction>,
    grids: ReachableGrids,
    mut overlay: ResMut<ReachableCells>,
) {
    let next = match resolve_selected(*selected, &actors) {
        Some((start, budget, mover_faction, factor)) => {
            reachable_for(start, budget, mover_faction, factor, &factions, &grids)
        }
        None => ReachableCells::cleared(),
    };
    // Mutate only on a real change (C4 change-detection hygiene).
    if *overlay != next {
        *overlay = next;
    }
}

/// Resolve the `(start, budget, mover_faction, factor)` from the current selection, or
/// [`None`] if nothing is selected or components are missing.
///
/// GTW-444: the `factor` is the selected ganger's
/// [`MovementCostFactor`] (the "Hampered" slowdown), read from its
/// [`InflictedInjuries`] ledger — [`MovementCostFactor::IDENTITY`] (`1.0`) when the
/// ganger has no ledger or no slowdown, so the overlay matches the commit's factor.
fn resolve_selected(
    selected: SelectedShooter,
    actors: &Query<(&Position, &Tu, &Faction, Option<&InflictedInjuries>)>,
) -> Option<(CellLevel, Tu, Faction, MovementCostFactor)> {
    let entity = (*selected)?;
    let (position, &budget, &mover_faction, injuries) = actors.get(entity).ok()?;
    let factor = injuries.map_or(
        MovementCostFactor::IDENTITY,
        InflictedInjuries::movement_cost_factor,
    );
    Some((**position, budget, mover_faction, factor))
}

/// The reachable set for `start` within `budget` TU — the same [`PlanningView`] +
/// [`reachable_within`] construction as `dispatch_move`'s planning path. Builds the fog
/// gate from the squad visibility + a per-call `mover_faction` resolver so the reachable
/// set respects the same fog rules as `find_path`.
fn reachable_for(
    start: CellLevel,
    budget: Tu,
    mover_faction: Faction,
    factor: MovementCostFactor,
    factions: &Query<&'static Faction>,
    grids: &ReachableGrids,
) -> ReachableCells {
    let planning = PlanningView::new(&grids.squad, |occupant| {
        relation_to(factions, mover_faction, occupant)
    });
    let raw = reachable_within(
        start,
        budget,
        &grids.grid,
        &grids.links,
        &grids.tuning,
        &grids.floor_costs,
        factor,
        &planning,
    );
    ReachableCells::new(raw)
}

/// The faction relation of `occupant` relative to `mover_faction` — same gang is
/// [`FactionRelation::OwnSquad`], any other (or an occupant with no [`Faction`]) is
/// [`FactionRelation::Other`].
///
/// A verbatim mirror of the equivalent resolver in `populate_path_preview` — the faction
/// resolver is identical because the planning view semantics must match the commit path.
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

/// The grids the reachable flood reads, bundled as one [`SystemParam`] — the same
/// [`OccupancyGrid`] + [`VerticalLinkGraph`] + [`SquadVisibility`] + [`CombatTuning`] +
/// [`FloorCostGrid`] the path-preview populate and `dispatch_move` use. Framework plumbing
/// (a borrow bundle), exempt from no-bare-types; bundling keeps
/// [`populate_reachable_overlay`] under the `too_many_arguments` lint.
#[derive(SystemParam)]
pub struct ReachableGrids<'w> {
    /// The coarse occupancy grid (terrain + occupants) the flood routes over.
    pub(crate) grid:        Res<'w, OccupancyGrid>,
    /// The vertical-link graph the flood stitches storeys through.
    pub(crate) links:       Res<'w, VerticalLinkGraph>,
    /// The squad fog the [`PlanningView`] gates routability by.
    pub(crate) squad:       Res<'w, SquadVisibility>,
    /// The combat tuning (the flat link cost + other combat coefficients).
    pub(crate) tuning:      Res<'w, CombatTuning>,
    /// The per-cell floor move-cost surface (GTW-396) — read by `reachable_within`
    /// instead of `tuning.move_costs` for every planar step cost.
    pub(crate) floor_costs: Res<'w, FloorCostGrid>,
}
