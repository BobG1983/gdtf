//! AC for the pure read seams: [`is_ganger_visible`](crate::visibility::is_ganger_visible)
//! (player always visible, enemy iff its cell is VISIBLE) and the set-lookup seams
//! [`is_cell_visible`](crate::visibility::SquadVisibility::is_cell_visible) /
//! [`is_cell_explored`](crate::visibility::SquadVisibility::is_cell_explored) (GTW-340
//! clauses 2 / 3 / sixth+seventh ACs).

use super::support::*;

/// A small squad fog: `visible_cell` is VISIBLE (and so explored); `explored_only_cell`
/// is EXPLORED but NOT visible; `unseen_cell` is in neither.
fn fixture() -> (SquadVisibility, CellLevel, CellLevel, CellLevel) {
    let visible_cell = key(5, 5, 0);
    let explored_only_cell = key(6, 6, 0);
    let unseen_cell = key(20, 20, 0);

    let mut visible = HashSet::default();
    visible.insert(visible_cell);
    let mut explored = HashSet::default();
    explored.insert(visible_cell);
    explored.insert(explored_only_cell);

    (
        SquadVisibility::new(visible, explored),
        visible_cell,
        explored_only_cell,
        unseen_cell,
    )
}

/// `is_cell_visible` / `is_cell_explored` are pure set lookups: VISIBLE ⊆ EXPLORED, an
/// explored-only cell reads explored-not-visible, an unseen cell reads neither — and the
/// signatures take ONLY a `&CellLevel` (no grid / tuning params, clause 2 / seventh AC).
#[test]
fn cell_read_seams_are_pure_set_lookups() {
    let (fog, visible_cell, explored_only_cell, unseen_cell) = fixture();

    // A VISIBLE cell is both visible and explored.
    assert!(fog.is_cell_visible(&visible_cell));
    assert!(fog.is_cell_explored(&visible_cell));

    // An EXPLORED-only cell is explored but not visible.
    assert!(!fog.is_cell_visible(&explored_only_cell));
    assert!(fog.is_cell_explored(&explored_only_cell));

    // An UNSEEN cell is in neither set.
    assert!(!fog.is_cell_visible(&unseen_cell));
    assert!(!fog.is_cell_explored(&unseen_cell));
}

/// `visible_cells` yields exactly the VISIBLE set — the iterator seam (clause 2).
#[test]
fn visible_cells_iterator_yields_the_visible_set() {
    let (fog, visible_cell, explored_only_cell, _unseen) = fixture();
    let cells: HashSet<CellLevel> = fog.visible_cells().copied().collect();
    assert!(
        cells.contains(&visible_cell),
        "the VISIBLE cell is iterated"
    );
    assert!(
        !cells.contains(&explored_only_cell),
        "an explored-only cell is NOT in the VISIBLE iterator"
    );
    assert_eq!(cells.len(), 1, "exactly the one VISIBLE cell is yielded");
}

/// `is_ganger_visible`: a player (own-squad) ganger is ALWAYS visible — even at an
/// unseen cell (you always see your own squad; clause 3 / sixth AC).
#[test]
fn player_ganger_is_always_visible() {
    let (fog, _visible, _explored, unseen_cell) = fixture();
    assert!(
        is_ganger_visible(&fog, &unseen_cell, FactionRelation::OwnSquad),
        "a player ganger is trivially visible regardless of the squad sets"
    );
}

/// `is_ganger_visible`: an ENEMY (other-faction) ganger is visible iff its cell is in
/// the VISIBLE set — true at a visible cell, false at an explored-only or unseen cell
/// (clause 3 / sixth AC).
#[test]
fn enemy_ganger_visible_iff_cell_is_visible() {
    let (fog, visible_cell, explored_only_cell, unseen_cell) = fixture();

    assert!(
        is_ganger_visible(&fog, &visible_cell, FactionRelation::Other),
        "an enemy at a squad-VISIBLE cell is visible"
    );
    assert!(
        !is_ganger_visible(&fog, &explored_only_cell, FactionRelation::Other),
        "an enemy at a merely-EXPLORED cell is NOT visible (only VISIBLE shows enemies)"
    );
    assert!(
        !is_ganger_visible(&fog, &unseen_cell, FactionRelation::Other),
        "an enemy at an UNSEEN cell is NOT visible"
    );
}
