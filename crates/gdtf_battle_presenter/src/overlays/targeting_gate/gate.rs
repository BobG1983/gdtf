//! Squad-visible cell gate for targeting UI.

use gdtf_battle_sim::{
    prelude::CellLevel,
    visibility::{FactionRelation, SquadVisibility, is_ganger_visible},
};

/// Whether a cell is currently squad-visible for targeting chrome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CellVisibility {
    /// Cell is in the squad's current visibility set.
    SquadVisible,
    /// Cell is unseen or only explored, not currently visible.
    NotSquadVisible,
}

impl CellVisibility {
    /// `true` when the cell is squad-visible.
    #[must_use]
    pub const fn is_squad_visible(self) -> bool {
        matches!(self, Self::SquadVisible)
    }
}

/// Resolve cell visibility for empty cells or occupied cells with a faction relation.
#[must_use]
pub fn cell_squad_visible(
    squad: Option<&SquadVisibility>,
    cell: &CellLevel,
    relation: Option<FactionRelation>,
) -> CellVisibility {
    let Some(squad) = squad else {
        return CellVisibility::NotSquadVisible;
    };
    let visible = match relation {
        Some(relation) => *is_ganger_visible(squad, cell, relation),
        None => *squad.is_cell_visible(cell),
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
    use gdtf_battle_sim::{
        prelude::{Cell, CellLevel, Level},
        visibility::{FactionRelation, SquadVisibility},
    };

    use super::{CellVisibility, cell_squad_visible};

    fn cell(x: i32, y: i32) -> CellLevel {
        CellLevel::new(Cell::new(x, y), Level::new(0))
    }

    fn fog(visible: &[CellLevel], explored: &[CellLevel]) -> SquadVisibility {
        let visible: HashSet<CellLevel> = visible.iter().copied().collect();
        let mut explored_set: HashSet<CellLevel> = explored.iter().copied().collect();
        explored_set.extend(visible.iter().copied());
        SquadVisibility::new(visible, explored_set)
    }

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

    #[test]
    fn explored_not_visible_is_distinct_and_refused() {
        let explored_only = cell(4, 4);
        let visible = cell(5, 5);
        let squad = fog(&[visible], &[explored_only]);
        assert!(
            *squad.is_cell_explored(&explored_only),
            "fixture: the cell must be EXPLORED (mission memory)",
        );
        assert!(
            !*squad.is_cell_visible(&explored_only),
            "fixture: the EXPLORED cell must NOT be currently VISIBLE",
        );
        assert_eq!(
            cell_squad_visible(Some(&squad), &explored_only, None),
            CellVisibility::NotSquadVisible,
            "an EXPLORED-not-VISIBLE cell is refused exactly like UNSEEN (C5)",
        );
        assert_eq!(
            cell_squad_visible(Some(&squad), &visible, None),
            CellVisibility::SquadVisible,
            "a currently-VISIBLE cell is accepted",
        );
    }

    #[test]
    fn occupant_relation_routes_through_is_ganger_visible() {
        let unseen = cell(8, 1);
        let visible = cell(1, 8);
        let squad = fog(&[visible], &[]);

        assert_eq!(
            cell_squad_visible(Some(&squad), &unseen, Some(FactionRelation::OwnSquad)),
            CellVisibility::SquadVisible,
            "your own squad is always visible (OwnSquad)",
        );
        assert_eq!(
            cell_squad_visible(Some(&squad), &unseen, Some(FactionRelation::Other)),
            CellVisibility::NotSquadVisible,
            "an enemy in an UNSEEN cell is NotSquadVisible",
        );
        assert_eq!(
            cell_squad_visible(Some(&squad), &visible, Some(FactionRelation::Other)),
            CellVisibility::SquadVisible,
            "an enemy in a VISIBLE cell is SquadVisible",
        );
    }
}
