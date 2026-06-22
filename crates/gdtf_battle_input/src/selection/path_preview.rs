//! The route path-preview seam + POPULATE system (E7 · GTW-12j, C6): the input-crate half
//! of the move-route preview.
//!
//! The presenter owns the [`PathPreview`] read-seam + the draw system; this module defines
//! the NEW [`PathPreviewTarget`] seam (the target cell the route previews TO) and POPULATES
//! the presenter resource for the SELECTED ganger → that target. It is the ONLY place
//! [`SelectedShooter`] + [`PathPreviewTarget`] feed the route preview — keeping selection +
//! target out of the authoritative sim model (the `input → presenter → sim` direction; the
//! presenter DEFINES the resource, this input crate WRITES it, the
//! [`ReachableOverlay`](gdtf_battle_presenter::ReachableOverlay) /
//! [`HighlightRequest`](gdtf_battle_presenter::HighlightRequest) precedent).
//!
//! It computes the route the SAME way the move dispatch
//! ([`dispatch_move`](gdtf_battle_sim::acts::dispatch_move)) + the GTW-357 reachable overlay
//! plan it: it builds the GTW-353 [`PlanningView`] from the sim's [`SquadVisibility`] with an
//! occupant→[`FactionRelation`] resolver closed over the selected ganger's faction, then calls
//! [`find_path`](gdtf_battle_sim::find_path) over the SAME grids — so the previewed route +
//! cost EXACTLY match what a commit will accept (the GTW-354 / GTW-355 dependency) and the
//! route NEVER enters an UNSEEN cell (AC2). It REUSES `find_path` — no re-implemented routing
//! or cost. On [`PathBlocked`](gdtf_battle_sim::PathBlocked) (unreachable, or only reachable
//! through UNSEEN) it clears the preview (C2).
//!
//! # The GTW-356 boundary (FLAGGED)
//!
//! THIS ticket DEFINES [`PathPreviewTarget`] (default [`None`]) + the populate + the presenter
//! draw, but does NOT set the target — that is the GTW-356 two-click handling (click-1 sets
//! [`PathPreviewTarget`], click-2 commits). [`PathPreviewTarget`] is `init_resource`-d here so
//! the populate system reads a present resource (its [`Default`] is the empty target → no
//! preview), and GTW-356 will write it.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_presenter::PathPreview;
use gdtf_battle_sim::{
    CellLevel, CombatTuning, Faction, FactionRelation, OccupancyGrid, PlanningView, Position,
    SquadVisibility, VerticalLinkGraph, find_path,
};

use crate::selection::resources::SelectedShooter;

/// The cell the route preview previews TO — the target the SELECTED ganger's route is drawn
/// toward (C6).
///
/// A named newtype over `Option<CellLevel>` (no-bare-types: the preview target is a domain
/// value; [`CellLevel`] is the wrapped sim metric) that [`Deref`]s to its inner [`Option`] so
/// a reader matches it directly. `init_resource`-d by
/// [`GdtfBattleInputPlugin`](crate::GdtfBattleInputPlugin) — so its [`Default`] is the empty
/// target ([`None`] → no preview).
///
/// # The GTW-356 boundary (FLAGGED)
///
/// THIS ticket DEFINES this seam (default [`None`]); the GTW-356 two-click flow SETS it on the
/// first click (target select). Until then it stays [`None`] and the preview is empty — the
/// populate path is fully wired and unit-tested by authoring this resource directly. It lives
/// in the INPUT crate (beside [`SelectedShooter`]) so selection + target both stay out of the
/// sim (the `input → presenter → sim` direction).
#[derive(Resource, Deref, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PathPreviewTarget(Option<CellLevel>);

impl PathPreviewTarget {
    /// Build a target holding `cell` — what the GTW-356 click-1 (target select) will set.
    #[must_use]
    pub const fn new(cell: CellLevel) -> Self {
        Self(Some(cell))
    }

    /// The empty (no-target) preview target — the [`Default`], and what a deselect / a click
    /// off any target clears to.
    #[must_use]
    pub const fn cleared() -> Self {
        Self(None)
    }
}

/// The faction relation of `occupant` relative to `mover_faction` — same gang is
/// [`FactionRelation::OwnSquad`], any other (or an occupant with no [`Faction`]) is
/// [`FactionRelation::Other`].
///
/// The [`PlanningView`] occupant→relation resolver, built per-rebuild from the selected
/// ganger's faction and the live `&`[`Faction`] query — the SAME resolver
/// [`populate_reachable_overlay`](crate::populate_reachable_overlay) +
/// `dispatch_move` use, so the preview's visibility gate is identical to the commit's. A
/// non-ganger occupant maps to [`FactionRelation::Other`] (the conservative classification:
/// blocked only when its cell is squad-VISIBLE).
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

/// The grids the route search reads, bundled as one [`SystemParam`] — the [`OccupancyGrid`] +
/// [`VerticalLinkGraph`] + [`SquadVisibility`] + [`CombatTuning`] the SAME `dispatch_move`
/// route gate reads. Framework plumbing (a borrow bundle), exempt from no-bare-types; bundling
/// them keeps [`populate_path_preview`] under the `too_many_arguments` / `too_many_lines`
/// lints (the [`RouteGrids`](crate::selection::reachable) precedent).
#[derive(SystemParam)]
pub struct PreviewGrids<'w> {
    /// The coarse occupancy grid (terrain + occupants) the route routes over.
    grid:   Res<'w, OccupancyGrid>,
    /// The vertical-link graph the route stitches storeys through.
    links:  Res<'w, VerticalLinkGraph>,
    /// The squad fog the GTW-353 [`PlanningView`] gates routability by.
    squad:  Res<'w, SquadVisibility>,
    /// The combat tuning (the [`MoveCosts`](gdtf_battle_sim::MoveCosts) table + link cost).
    tuning: Res<'w, CombatTuning>,
}

/// `Update` ([`InputSystems::Gather`](crate::InputSystems)): POPULATE the presenter-owned
/// [`PathPreview`] for the [`SelectedShooter`] → the [`PathPreviewTarget`] — the
/// [`find_path`](gdtf_battle_sim::find_path) route over the visibility-gated grid + its §48
/// total cost (C1 / C6).
///
/// When a ganger is selected, a target is set, and the ganger's `(`[`Position`]`,
/// `[`Faction`]`)` resolves, it delegates to [`route_for`] (the SAME `PlanningView` +
/// `find_path` construction `dispatch_move` plans with) and writes the route + cost; otherwise
/// (no selection, no target, missing component, OR [`PathBlocked`](gdtf_battle_sim::PathBlocked)
/// — unreachable / only-through-UNSEEN) it writes [`PathPreview::cleared`] (the empty preview,
/// C2). It writes only on a real CHANGE (the `!=` guard) so an unchanged selection / target /
/// grid does not spuriously trip `Changed<PathPreview>` (C4 clears-on-change hygiene). REUSES
/// [`find_path`](gdtf_battle_sim::find_path) — no re-implemented routing, selection + target
/// NEVER pushed into the sim. Param-only (`bevy-traps.md` #7).
pub fn populate_path_preview(
    selected: Res<SelectedShooter>,
    target: Res<PathPreviewTarget>,
    actors: Query<(&Position, &Faction)>,
    factions: Query<&'static Faction>,
    grids: PreviewGrids,
    mut preview: ResMut<PathPreview>,
) {
    let next = match resolve_inputs(*selected, *target, &actors) {
        Some((start, goal, mover_faction)) => {
            route_for(start, goal, mover_faction, &factions, &grids)
        }
        // Nothing selected, no target, or a missing component → clear the preview.
        None => PathPreview::cleared(),
    };

    // Mutate only on a real change (C4 clears-on-change / change-detection hygiene).
    if *preview != next {
        *preview = next;
    }
}

/// Resolve the `(start, goal, mover_faction)` the preview needs from the current selection +
/// target, or [`None`] if any is absent.
///
/// `None` when nothing is selected, no target is set, or the selected entity has no
/// `(`[`Position`]`, `[`Faction`]`)` — every one of which means "no route to preview".
fn resolve_inputs(
    selected: SelectedShooter,
    target: PathPreviewTarget,
    actors: &Query<(&Position, &Faction)>,
) -> Option<(CellLevel, CellLevel, Faction)> {
    let entity = (*selected)?;
    let goal = (*target)?;
    let (position, &mover_faction) = actors.get(entity).ok()?;
    Some((**position, goal, mover_faction))
}

/// The path preview for a ganger at `start` routing to `goal` with `mover_faction` — the SAME
/// visibility-gated [`PlanningView`] + [`find_path`](gdtf_battle_sim::find_path) construction
/// [`dispatch_move`](gdtf_battle_sim::acts::dispatch_move) plans a route with, so the previewed
/// route + cost EXACTLY match what a commit will accept (C1) and the route NEVER enters an
/// UNSEEN cell (C2).
///
/// Builds the [`PlanningView`] from the squad fog + the per-call `relation_to` resolver (closed
/// over `mover_faction` + the live `&`[`Faction`] query), then routes from `start` to `goal`
/// over the [`PreviewGrids`]. On [`PathBlocked`](gdtf_battle_sim::PathBlocked) (unreachable, or
/// only reachable through UNSEEN) it returns the cleared preview (C2). On success the previewed
/// cost is [`Path::total`](gdtf_battle_sim::Path::total) — the §48 bit-identity GTW-355 charges.
/// REUSES the sim search — no re-implemented routing / cost.
fn route_for(
    start: CellLevel,
    goal: CellLevel,
    mover_faction: Faction,
    factions: &Query<&'static Faction>,
    grids: &PreviewGrids,
) -> PathPreview {
    // UNSEEN cells are non-routable; a visible enemy / own-squad ganger blocks; EXPLORED stays
    // routable — the GTW-353 gate, identical to the commit's.
    let planning = PlanningView::new(&grids.squad, |occupant| {
        relation_to(factions, mover_faction, occupant)
    });
    match find_path(
        start,
        goal,
        &grids.grid,
        &grids.links,
        &grids.tuning,
        &planning,
    ) {
        // The route cells (start..=goal in step order) + the §48 total cost.
        Ok(path) => PathPreview::new(path.cells().to_vec(), path.total()),
        // No route (unreachable, or only reachable through UNSEEN) → no preview (C2).
        Err(_blocked) => PathPreview::cleared(),
    }
}
