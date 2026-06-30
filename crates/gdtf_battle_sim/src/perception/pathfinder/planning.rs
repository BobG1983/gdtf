//! The **visibility-aware planning gate** (E7 · GTW-12e, ADR-0005; the
//! user-ratified OQ-5 ruling 2026-06-22) — the pure predicate that decides whether a
//! candidate `(cell, level)` is ROUTABLE for the path search, folding the squad
//! fog-of-war ([`SquadVisibility`]) into the occupancy-grid blocking truth.
//!
//! ## The OQ-5 interpretation this builds (C1)
//!
//! The user ruling REVERSES the old "plan through the dark" canon
//! (`docs/combat/visibility.md` §42 / §52): the search MUST NOT route INTO cells the
//! squad cannot currently SEE. The interpretation built here — flagged for the
//! orchestrator and documented in `visibility.md` — resolves "areas you can't see" as
//! **UNSEEN (never seen)**, NOT merely-explored:
//!
//! - **UNSEEN** (in neither the VISIBLE nor the EXPLORED set) is **non-routable** —
//!   treated as impassable for planning, so [`find_path`](super::find_path) routes
//!   AROUND it within the non-UNSEEN set and
//!   [`reachable_within`](super::reachable_within) never yields it.
//! - **EXPLORED** (remembered, seen at some point this mission) **remains routable** —
//!   the XCOM model: you can plan through what you remember, just not the true dark.
//! - So **ROUTABLE = non-UNSEEN = VISIBLE ∪ EXPLORED**. A goal reachable ONLY by
//!   crossing UNSEEN is [`PathBlocked`](super::PathBlocked).
//!
//! Because the accrual invariant keeps `VISIBLE ⊆ EXPLORED`
//! ([`accrue`](crate::visibility::accrue)), the non-UNSEEN test is exactly
//! [`SquadVisibility::is_cell_explored`] — a cell is non-UNSEEN iff it is EXPLORED.
//!
//! ## The within-routable blocking predicate (C2)
//!
//! On a routable cell the blocking rule is visibility-aware, reusing the GTW-13
//! [`is_ganger_visible`] read seam (`visibility.md` §40):
//!
//! - **Walls + floor geometry ALWAYS block** regardless of visibility — true geometry
//!   (`visibility.md` §42): [`OccupancyGrid::is_blocked`] (a wall, or standing,
//!   non-destroyed cover). Blocking scatter / props are the model's
//!   [`Cover`](crate::occupancy::TerrainKind::Cover) terrain and so are covered by the
//!   same `is_blocked` read; on a non-UNSEEN (routable) cell they block (§41), and on
//!   an UNSEEN cell the cell is non-routable anyway, so the §41 "non-UNSEEN only"
//!   bound is satisfied by construction.
//! - **An OWN-SQUAD ganger ALWAYS blocks** (you always see your own — `is_ganger_visible`
//!   with [`FactionRelation::OwnSquad`] is `true`).
//! - **An ENEMY ganger blocks IFF its cell is squad-VISIBLE** (`is_ganger_visible` with
//!   [`FactionRelation::Other`]) — an enemy the squad cannot currently see never bends
//!   the route (bending around an unseen body would leak its position).
//!
//! ## GTW-387: the scoped UNSEEN relaxation for vertical link far endpoints
//!
//! A cell that is UNSEEN is normally non-routable (C1). The **one exception** is a
//! cell that is the FAR ENDPOINT of a vertical link departing the cell currently being
//! expanded: you may plan ONTO an as-yet-unseen storey via a **known** stair or ladder,
//! because the link was authored and validated at setup — the
//! [`VerticalLinkGraph`](crate::vertical::VerticalLinkGraph) is the "known-link" oracle.
//! This relaxation is applied by [`PlanningView::is_routable_link`] — a C2-only
//! predicate used ONLY for the `traversable_links` (vertical) half of `edges()` in the
//! search core. Planar 8-connected steps still keep the full C1+C2 gate
//! ([`PlanningView::is_routable`]), so there is no see-through-walls or
//! route-through-dark hole. Once the ganger arrives at the link head the FOV re-computes
//! and the platform cells on the far storey become EXPLORED and plannable on the next
//! tick — the OQ-5 invariant for non-link cells is unchanged.
//!
//! ## Purity + determinism (C4 / C5)
//!
//! The gate is a PURE deterministic predicate over the borrowed [`SquadVisibility`]
//! sets + the [`OccupancyGrid`] + a caller-supplied occupant→[`FactionRelation`]
//! resolver — no RNG, no `&mut World`, no nondeterministic iteration (set membership
//! and grid lookups only). It changes WHICH edges the search accepts, never the
//! octile / link COSTS or the deterministic `(z, y, x)` tie-break (those stay intact).

use bevy::prelude::Entity;

use crate::{
    metric::CellLevel,
    occupancy::OccupancyGrid,
    visibility::{FactionRelation, SquadVisibility, is_ganger_visible},
};

/// The borrowed squad-fog view + occupant-faction resolver the planning gate reads as
/// a SNAPSHOT — the visibility-aware blocking seam threaded into both search entry
/// points (C2).
///
/// A read-only view bundling the GTW-13 [`SquadVisibility`] (the VISIBLE / EXPLORED
/// sets) with `relation_of`, the caller's map from an occupant [`Entity`] to its
/// [`FactionRelation`] to the player squad. The resolver is supplied by the caller
/// because the occupancy grid stores only the occupant `Entity` handle (never a
/// faction), and the faction lives on the ganger's components — so the seam stays
/// pure (no world access) by taking the lookup as a closure. It is framework plumbing
/// (a borrow bundle + a resolver), not a domain value.
///
/// `R` is the resolver closure type; an occupant whose entity is not a live faction
/// ganger (a non-ganger occupant, or one the caller chooses not to treat as a blocker)
/// can be mapped to whichever relation the caller wants — the gate only asks the
/// resolver for a relation when a cell actually HAS an occupant.
pub struct PlanningView<'a, R>
where
    R: Fn(Entity) -> FactionRelation,
{
    /// The squad three-state fog — the VISIBLE / EXPLORED sets the gate reads to
    /// decide routability (UNSEEN = the implicit complement) and enemy visibility.
    squad:       &'a SquadVisibility,
    /// The caller's occupant `Entity` → [`FactionRelation`] resolver — own-squad vs
    /// other-faction relative to the player squad. Asked only for occupied cells.
    relation_of: R,
}

impl<'a, R> PlanningView<'a, R>
where
    R: Fn(Entity) -> FactionRelation,
{
    /// Build a planning view from the squad fog and an occupant-faction resolver — the
    /// visibility-aware blocking seam the search threads (C2).
    #[must_use]
    pub const fn new(squad: &'a SquadVisibility, relation_of: R) -> Self {
        Self { squad, relation_of }
    }

    /// Whether `cell` is **routable** for planning — non-UNSEEN AND not blocked
    /// within-routable (C1 + C2). A candidate the search REJECTS when this is `false`.
    ///
    /// Two gates, in order:
    ///
    /// 1. **C1 — non-UNSEEN.** A cell the squad has never seen (in neither VISIBLE nor
    ///    EXPLORED) is impassable for planning: the search routes around it. EXPLORED
    ///    remains routable (the OQ-5 interpretation). The non-UNSEEN test is
    ///    [`SquadVisibility::is_cell_explored`] because `VISIBLE ⊆ EXPLORED`.
    /// 2. **C2 — within-routable blocking.** On a non-UNSEEN cell:
    ///    - walls / floor / standing cover (blocking scatter/props) ALWAYS block —
    ///      [`OccupancyGrid::is_blocked`] (true geometry, §42 / §41);
    ///    - an occupant blocks per [`is_ganger_visible`] of its
    ///      [`FactionRelation`]: own-squad always, an enemy iff its cell is
    ///      squad-VISIBLE (§40).
    ///
    /// PURE: set membership + grid lookups + the resolver — no RNG, no world access.
    #[must_use]
    pub fn is_routable(&self, cell: CellLevel, grid: &OccupancyGrid) -> bool {
        // C1: UNSEEN (never seen) is non-routable. Non-UNSEEN == EXPLORED (VISIBLE is a
        // subset of EXPLORED by the accrual invariant), so EXPLORED stays routable.
        if !self.squad.is_cell_explored(&cell) {
            return false;
        }
        // C2 — shared blocking check (geometry + visibility-aware occupant).
        self.is_open(cell, grid)
    }

    /// Routability for a cell reached over a **known vertical link** (GTW-387) — C2
    /// only.
    ///
    /// The C1 UNSEEN gate is **lifted** for cells that arrive via the validated
    /// [`VerticalLinkGraph`](crate::vertical::VerticalLinkGraph): a ganger may PLAN
    /// onto an as-yet-unseen storey by following a known stair or ladder, because the
    /// link itself was authored and validated at setup — the graph is the
    /// "known-link" oracle. ONLY the fog-explored check (C1) is skipped; the full C2
    /// geometry and occupant block still applies.
    ///
    /// **The relaxation does NOT widen to arbitrary UNSEEN cells:** it is applied
    /// only to cells emitted by [`traversable_links`](crate::vertical::traversable_links)
    /// (i.e. far endpoints of links departing the currently-expanded origin) — planar
    /// 8-connected neighbours keep the full [`is_routable`](Self::is_routable) gate, so
    /// there is no see-through-walls or route-through-dark hole. An UNSEEN floor tile
    /// that is not a link endpoint is never enqueued. Once the ganger arrives at the
    /// (UNSEEN) link head and FOV re-computes, the platform cells on the far storey
    /// become EXPLORED and are plannable on the next tick — the `input → presenter → sim`
    /// multi-tick play model (OQ-5 invariant for non-link cells is unchanged).
    ///
    /// PURE: grid lookups + the resolver — no RNG, no world access.
    #[must_use]
    pub fn is_routable_link(&self, cell: CellLevel, grid: &OccupancyGrid) -> bool {
        // C1 — SKIPPED for a known link endpoint. The vertical graph is the oracle:
        // if this cell came from traversable_links it IS a known stair/ladder endpoint,
        // so planning onto an unseen storey via it is permitted.

        // C2 — geometry + occupant blocking still apply unconditionally.
        self.is_open(cell, grid)
    }

    /// The shared C2 blocking test — walls / cover geometry ALWAYS block, and an
    /// occupant blocks per [`is_ganger_visible`] — used by both [`is_routable`] and
    /// [`is_routable_link`] so the rule lives in ONE place and cannot drift.
    ///
    /// PURE: grid lookups + the resolver — no RNG, no world access.
    fn is_open(&self, cell: CellLevel, grid: &OccupancyGrid) -> bool {
        // C2 (true geometry): walls / floor / standing cover (blocking scatter/props)
        // always block on a routable cell. GTW-501 D1/C2: this is the PATHFINDER's gate,
        // so it reads the TAG-DERIVED path-blocking surface (`is_path_blocked`) — the
        // markers projected from `BlocksPathfinding`, NOT the kind-based `is_blocked` that
        // vision reads. The surface mirrors the destroyed-cover exclusion (C5), so a
        // destroyed wall/cover re-opens the route exactly as before.
        if grid.is_path_blocked(&cell) {
            return false;
        }
        // C2 (visibility-aware occupant): an occupant blocks per its relation —
        // own-squad always, an enemy only when its cell is squad-VISIBLE.
        if let Some(occupant) = grid.occupant(&cell) {
            let relation = (self.relation_of)(occupant);
            if is_ganger_visible(self.squad, &cell, relation) {
                return false;
            }
        }
        true
    }
}
