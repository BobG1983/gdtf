//! The squad three-state fog model — the [`SquadVisibility`] resource (the
//! VISIBLE / EXPLORED sets, UNSEEN the implicit complement) and its **pure read
//! seams** (GTW-340, leaf 4 of the GTW-13 FOV epic).
//!
//! These are the seams the consumers read — the GTW-11 fog gate, the GTW-70 AI, and
//! the GTW-38 reaction fire — never recompute logic (that is GTW-341). Every read
//! here is a set lookup: no geometry, no march, no grid / tuning input.

use bevy::{platform::collections::HashSet, prelude::Resource};

use crate::{metric::CellLevel, occupancy::OccupancyGrid};

/// Whether a `(cell, level)` is the **player squad's**: own-squad (always shown) or
/// an other-faction relation (shown only when the cell is squad-VISIBLE) — the
/// faction relation [`is_ganger_visible`] takes as its explicit predicate (GTW-340
/// clause 3).
///
/// A named domain enum (no-bare-types: the friend/foe relation of a watched ganger
/// to the player squad is a domain value, not a bare boolean). It is the EXACT
/// consumption seam the "rendered-only planning" reads use
/// (`docs/combat/visibility.md` §"Rendered-only planning"): GTW-230's walk-time
/// **bump-stop** and the pathfinder's **enemy-route-blocking** read both ask
/// [`is_ganger_visible`] whether a ganger should block / show, passing the relation of
/// that ganger to the player squad.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FactionRelation {
    /// A **player-squad** ganger — trivially visible (you always see your own), so
    /// [`is_ganger_visible`] returns `true` regardless of the squad sets.
    OwnSquad,
    /// A **non-player** ganger — visible only when its `(cell, level)` is in the
    /// squad-VISIBLE set (the same read that shows / hides its entity).
    Other,
}

/// The player squad's three-state fog model — the per-`(cell, level)` VISIBLE and
/// EXPLORED sets, with UNSEEN the **implicit complement** of EXPLORED
/// (`docs/combat/visibility.md` §"The three states").
///
/// A Bevy [`Resource`] (one squad fog per battle), computed model-side: the squad
/// VISIBLE set is the **union** of every conscious player-faction observer's FOV
/// ([`union_fov`](crate::visibility::union_fov)), EXPLORED **accrues** from VISIBLE and
/// is **monotone per mission** — it only ever grows, surviving every recompute
/// ([`accrue`](crate::visibility::accrue)) — and UNSEEN is "in neither set" (never
/// stored). This leaf owns the resource + the read API; the system that recomputes and
/// writes it on each trigger is GTW-341.
///
/// The two inner sets are private (house style for a set-carrying domain value); they
/// are read ONLY through the pure seams below
/// ([`is_cell_visible`](SquadVisibility::is_cell_visible) /
/// [`is_cell_explored`](SquadVisibility::is_cell_explored) /
/// [`visible_cells`](SquadVisibility::visible_cells)) and replaced wholesale by the
/// accrual helper — never reached into directly.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct SquadVisibility {
    /// The cells some conscious player-faction ganger sees **right now** — the union
    /// of every observer's FOV. Replaced wholesale on each recompute.
    visible:  HashSet<CellLevel>,
    /// The cells seen at **some point this mission** — **monotone**: it only ever
    /// grows (`explored ∪= visible` on every recompute), so a cell that leaves VISIBLE
    /// stays EXPLORED. Never shrinks; there is no API on this type that removes a cell.
    explored: HashSet<CellLevel>,
}

impl SquadVisibility {
    /// Build a squad fog from its VISIBLE and EXPLORED sets — the constructor the
    /// accrual helper ([`accrue`](crate::visibility::accrue)) and the GTW-341 recompute
    /// system build a fresh fog through (keeping the inner sets private).
    ///
    /// Callers are expected to pass a `visible ⊆ explored` pair (the accrual invariant);
    /// this constructor does not re-derive the union — it is the one place the two sets
    /// are set together.
    #[must_use]
    pub const fn new(visible: HashSet<CellLevel>, explored: HashSet<CellLevel>) -> Self {
        Self { visible, explored }
    }

    /// An **omniscient** fog over `grid` — every in-bounds `(cell, level)` of the grid's
    /// fixed extent both VISIBLE and EXPLORED (GTW-70).
    ///
    /// The AI's MOVE-planning fog: a [`SquadVisibility`] whose VISIBLE and EXPLORED sets
    /// are the WHOLE `GRID_WIDTH × GRID_HEIGHT × MAX_LEVELS` cell set
    /// ([`OccupancyGrid::all_cells`]). Under it the [`PlanningView`](crate::pathfinder::PlanningView)
    /// gate is a no-op on routability — every cell is EXPLORED (so non-UNSEEN → routable)
    /// AND VISIBLE (so every occupant blocks) — which is exactly what lets the enemy AI
    /// navigate toward the player's TRUE cell while still colliding correctly. It does NOT
    /// let the AI SHOOT: firing gates on the real per-pair `can_see`, not this fog (GTW-70
    /// §D.1; the symmetric enemy fog-of-war is deferred to GTW-71). Built once at battle
    /// setup and held in the [`OmniscientFog`](crate::visibility::OmniscientFog) resource.
    ///
    /// `grid`'s extent is fixed once built, so this depends only on the grid's structural
    /// dimensions, never its terrain/occupant contents.
    #[must_use]
    pub fn omniscient(grid: &OccupancyGrid) -> Self {
        let all: HashSet<CellLevel> = grid.all_cells().collect();
        Self::new(all.clone(), all)
    }

    /// Whether `cell` is **squad-VISIBLE** — some conscious player-faction ganger sees
    /// it right now. A pure set lookup: no geometry, no recompute, no grid / tuning
    /// input (GTW-340 clause 2).
    #[must_use]
    pub fn is_cell_visible(&self, cell: &CellLevel) -> bool {
        self.visible.contains(cell)
    }

    /// Whether `cell` is **squad-EXPLORED** — seen at some point this mission (it may or
    /// may not also be VISIBLE now; EXPLORED is the superset). A pure set lookup: no
    /// geometry, no recompute, no grid / tuning input (GTW-340 clause 2).
    #[must_use]
    pub fn is_cell_explored(&self, cell: &CellLevel) -> bool {
        self.explored.contains(cell)
    }

    /// The squad-VISIBLE cells — the iterator/query seam over the VISIBLE set (GTW-340
    /// clause 2). A pure read: it borrows the set and yields each currently-seen
    /// `(cell, level)`, in no defined order (a `HashSet`). The fog presenter (GTW-342)
    /// and any consumer that needs to walk what the squad sees read it through this.
    pub fn visible_cells(&self) -> impl Iterator<Item = &CellLevel> + '_ {
        self.visible.iter()
    }

    /// The squad-EXPLORED cells — the iterator/query seam over the (monotone) EXPLORED
    /// set. A pure read mirroring [`visible_cells`](SquadVisibility::visible_cells): it
    /// borrows the set and yields each ever-seen `(cell, level)`, in no defined order.
    /// The accrual fold ([`accrue`](crate::visibility::accrue)) reads the previous
    /// mission memory back through this to grow it.
    pub fn explored_cells(&self) -> impl Iterator<Item = &CellLevel> + '_ {
        self.explored.iter()
    }
}

/// Whether a watched ganger at `target` should show / block for the player squad —
/// the **rendered-only planning** read (GTW-340 clause 3;
/// `docs/combat/visibility.md` §"Rendered-only planning").
///
/// A **pure read** over the squad VISIBLE set plus the explicit `relation`:
///
/// * a [`FactionRelation::OwnSquad`] ganger is **trivially visible** — you always see
///   your own squad, so the result is `true` regardless of the sets;
/// * a [`FactionRelation::Other`] ganger is visible **iff** its `target` `(cell, level)`
///   is squad-VISIBLE — the SAME read [`SquadVisibility::is_cell_visible`] performs that
///   shows / hides its entity, so plan and render can never disagree.
///
/// `relation` is the explicit faction-relation predicate the spec pins as the
/// consumption seam: GTW-230's walk-time **bump-stop** (an enemy ahead reveals iff the
/// mover's sight reaches it) and the **pathfinder's** enemy-route-blocking read call
/// this with the ganger's relation to the player squad. No geometry, no recompute — a
/// set lookup gated by the relation.
#[must_use]
pub fn is_ganger_visible(
    squad: &SquadVisibility,
    target: &CellLevel,
    relation: FactionRelation,
) -> bool {
    match relation {
        // Your own squad is always shown (player ids are trivially visible).
        FactionRelation::OwnSquad => true,
        // An enemy shows iff its cell is squad-VISIBLE — the same read that
        // shows/hides its entity, so plan and render agree.
        FactionRelation::Other => squad.is_cell_visible(target),
    }
}
