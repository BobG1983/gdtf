//! The [`CellVisibility`] verdict + the shared [`cell_squad_visible`] predicate.

use gdtf_battle_sim::{CellLevel, FactionRelation, SquadVisibility, is_ganger_visible};

/// Whether a targeted `(cell, level)` is currently squad-VISIBLE — the verdict the
/// reticle, the hint, AND the fire-refusal all read so they can never disagree
/// (`docs/combat/visibility.md` §"UX edges").
///
/// A NAMED domain enum (no-bare-types: the squad-visible verdict crosses the
/// input→presenter edge inside [`HighlightRequest`](crate::HighlightRequest) and is read
/// by the app's hint — it is a domain value, not a bare boolean). The two NON-VISIBLE
/// fog states (UNSEEN and merely EXPLORED) BOTH collapse to
/// [`NotSquadVisible`](CellVisibility::NotSquadVisible): the gate refuses fire and recolours
/// the reticle into ANY non-VISIBLE cell, EXPLORED no differently from UNSEEN
/// (`docs/combat/visibility.md` §"UX edges": "any non-VISIBLE cell (UNSEEN *or* merely
/// explored)"). Only a currently-squad-VISIBLE cell is [`SquadVisible`](CellVisibility::SquadVisible).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CellVisibility {
    /// The cell is squad-VISIBLE right now — fire is allowed, the reticle keeps its
    /// normal tint, and the hint is suppressed.
    SquadVisible,
    /// The cell is NOT squad-VISIBLE — it is UNSEEN *or* merely EXPLORED (both refuse
    /// identically). Fire is refused (the [`NoOp`](crate::HighlightRequest) rung), the
    /// reticle recolours, and the hint reads the canon "unseen — hold your fire".
    NotSquadVisible,
}

impl CellVisibility {
    /// Whether this verdict permits firing / a normal reticle (i.e. the cell is
    /// currently squad-VISIBLE). The single boolean read the three consumers branch on,
    /// keeping the "is it squad-visible" question in ONE place.
    #[must_use]
    pub const fn is_squad_visible(self) -> bool {
        matches!(self, Self::SquadVisible)
    }
}

/// The ONE shared squad-visible predicate — decides the [`CellVisibility`] of a targeted
/// `(cell, level)` from the squad fog, FAIL-CLOSED (`docs/combat/visibility.md` §"UX edges":
/// hint and act share one read so they can never disagree).
///
/// Reads the squad fog as `squad: Option<&`[`SquadVisibility`]`>` so the absent-resource
/// case is **fail-closed**: with no fog (e.g. a harness with no sim plugin, or a frame
/// before the battle seeds it) the cell is treated as NOT squad-VISIBLE
/// ([`CellVisibility::NotSquadVisible`]) — never permitting fire into a cell the squad's
/// truth cannot vouch for.
///
/// The verdict resolution (mirroring the sim's
/// [`is_ganger_visible`](gdtf_battle_sim::is_ganger_visible) read so plan and render agree):
///
/// * an **occupant-bearing** cell (the caller passes `relation = Some(..)` with the
///   occupant's [`FactionRelation`] to the player squad) is decided by
///   [`is_ganger_visible`]: a [`FactionRelation::OwnSquad`] occupant is trivially visible
///   (you always see your own), a [`FactionRelation::Other`] occupant is visible iff its
///   cell is squad-VISIBLE — the SAME read that shows / hides its sprite. The fire-refusal
///   only ever targets ENEMY cells, so it passes [`FactionRelation::Other`].
/// * an **empty / terrain** cell (`relation = None`) is decided by
///   [`SquadVisibility::is_cell_visible`] directly.
///
/// Pure (no `&mut World`, no system params): a borrow of the optional fog + the cell + the
/// optional occupant relation. The reticle ([`emit_highlight_request`](crate::HighlightRequest)
/// caller), the hint, and the fire-refusal rung all call this one fn.
#[must_use]
pub fn cell_squad_visible(
    squad: Option<&SquadVisibility>,
    cell: &CellLevel,
    relation: Option<FactionRelation>,
) -> CellVisibility {
    // FAIL-CLOSED: no squad fog -> never squad-visible (refuse / recolour / hint).
    let Some(squad) = squad else {
        return CellVisibility::NotSquadVisible;
    };
    let visible = match relation {
        // An occupant cell: the SAME read that shows / hides the occupant's sprite.
        Some(relation) => is_ganger_visible(squad, cell, relation),
        // An empty / terrain cell: a plain VISIBLE-set lookup (EXPLORED is NOT visible).
        None => squad.is_cell_visible(cell),
    };
    if visible {
        CellVisibility::SquadVisible
    } else {
        CellVisibility::NotSquadVisible
    }
}

#[cfg(test)]
mod tests {
    use bevy::platform::collections::HashSet;
    use gdtf_battle_sim::{Cell, CellLevel, FactionRelation, Level, SquadVisibility};

    use super::{CellVisibility, cell_squad_visible};

    /// A cell on the ground level at `(x, y)`.
    fn cell(x: i32, y: i32) -> CellLevel {
        CellLevel::new(Cell::new(x, y), Level::new(0))
    }

    /// A squad fog with the given VISIBLE + EXPLORED cells (EXPLORED is the superset).
    fn fog(visible: &[CellLevel], explored: &[CellLevel]) -> SquadVisibility {
        let visible: HashSet<CellLevel> = visible.iter().copied().collect();
        let mut explored_set: HashSet<CellLevel> = explored.iter().copied().collect();
        // The accrual invariant: VISIBLE ⊆ EXPLORED.
        explored_set.extend(visible.iter().copied());
        SquadVisibility::new(visible, explored_set)
    }

    /// FAIL-CLOSED: an absent squad fog yields `NotSquadVisible` — never permits fire.
    #[test]
    fn absent_fog_is_not_squad_visible() {
        assert_eq!(
            cell_squad_visible(None, &cell(3, 3), None),
            CellVisibility::NotSquadVisible,
            "no squad fog must fail closed to NotSquadVisible",
        );
        assert_eq!(
            cell_squad_visible(None, &cell(3, 3), Some(FactionRelation::Other)),
            CellVisibility::NotSquadVisible,
            "no squad fog must fail closed even for an enemy-relation cell",
        );
    }

    /// A VISIBLE empty cell is `SquadVisible`; an UNSEEN one (in neither set) is not.
    #[test]
    fn empty_cell_visible_vs_unseen() {
        let visible = cell(2, 2);
        let unseen = cell(9, 9);
        let squad = fog(&[visible], &[]);
        assert_eq!(
            cell_squad_visible(Some(&squad), &visible, None),
            CellVisibility::SquadVisible,
            "a VISIBLE empty cell is SquadVisible",
        );
        assert_eq!(
            cell_squad_visible(Some(&squad), &unseen, None),
            CellVisibility::NotSquadVisible,
            "an UNSEEN empty cell is NotSquadVisible",
        );
    }

    /// C5 — EXPLORED-but-not-VISIBLE is a DISTINCT case that is STILL `NotSquadVisible`,
    /// exactly like UNSEEN (the gate refuses fire into any non-VISIBLE cell).
    #[test]
    fn explored_not_visible_is_distinct_and_refused() {
        let explored_only = cell(4, 4);
        let visible = cell(5, 5);
        // `explored_only` is EXPLORED (mission memory) but NOT in the VISIBLE set.
        let squad = fog(&[visible], &[explored_only]);
        // Pin the DISTINCT EXPLORED state: the cell IS explored, is NOT visible.
        assert!(
            squad.is_cell_explored(&explored_only),
            "fixture: the cell must be EXPLORED (mission memory)",
        );
        assert!(
            !squad.is_cell_visible(&explored_only),
            "fixture: the EXPLORED cell must NOT be currently VISIBLE",
        );
        assert_eq!(
            cell_squad_visible(Some(&squad), &explored_only, None),
            CellVisibility::NotSquadVisible,
            "an EXPLORED-not-VISIBLE cell is refused exactly like UNSEEN (C5)",
        );
        // And the VISIBLE cell is accepted (the discriminating positive).
        assert_eq!(
            cell_squad_visible(Some(&squad), &visible, None),
            CellVisibility::SquadVisible,
            "a currently-VISIBLE cell is accepted",
        );
    }

    /// An occupant cell is decided by `is_ganger_visible`: an `OwnSquad` occupant is
    /// trivially visible even in an UNSEEN cell; an Other occupant follows the cell's
    /// VISIBLE state.
    #[test]
    fn occupant_relation_routes_through_is_ganger_visible() {
        let unseen = cell(8, 1);
        let visible = cell(1, 8);
        let squad = fog(&[visible], &[]);

        // OwnSquad: trivially visible regardless of the sets.
        assert_eq!(
            cell_squad_visible(Some(&squad), &unseen, Some(FactionRelation::OwnSquad)),
            CellVisibility::SquadVisible,
            "your own squad is always visible (OwnSquad)",
        );
        // Other in an UNSEEN cell: not visible.
        assert_eq!(
            cell_squad_visible(Some(&squad), &unseen, Some(FactionRelation::Other)),
            CellVisibility::NotSquadVisible,
            "an enemy in an UNSEEN cell is NotSquadVisible",
        );
        // Other in a VISIBLE cell: visible.
        assert_eq!(
            cell_squad_visible(Some(&squad), &visible, Some(FactionRelation::Other)),
            CellVisibility::SquadVisible,
            "an enemy in a VISIBLE cell is SquadVisible",
        );
    }
}
