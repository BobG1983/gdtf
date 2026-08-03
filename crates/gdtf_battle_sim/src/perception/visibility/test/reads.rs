use super::support::*;

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

#[test]
fn cell_read_seams_are_pure_set_lookups() {
    let (fog, visible_cell, explored_only_cell, unseen_cell) = fixture();

    assert!(*fog.is_cell_visible(&visible_cell));
    assert!(*fog.is_cell_explored(&visible_cell));

    assert!(!*fog.is_cell_visible(&explored_only_cell));
    assert!(*fog.is_cell_explored(&explored_only_cell));

    assert!(!*fog.is_cell_visible(&unseen_cell));
    assert!(!*fog.is_cell_explored(&unseen_cell));
}

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

#[test]
fn player_ganger_is_always_visible() {
    let (fog, _visible, _explored, unseen_cell) = fixture();
    assert!(
        *is_ganger_visible(&fog, &unseen_cell, FactionRelation::OwnSquad),
        "a player ganger is trivially visible regardless of the squad sets"
    );
}

#[test]
fn enemy_ganger_visible_iff_cell_is_visible() {
    let (fog, visible_cell, explored_only_cell, unseen_cell) = fixture();

    assert!(
        *is_ganger_visible(&fog, &visible_cell, FactionRelation::Other),
        "an enemy at a squad-VISIBLE cell is visible"
    );
    assert!(
        !*is_ganger_visible(&fog, &explored_only_cell, FactionRelation::Other),
        "an enemy at a merely-EXPLORED cell is NOT visible (only VISIBLE shows enemies)"
    );
    assert!(
        !*is_ganger_visible(&fog, &unseen_cell, FactionRelation::Other),
        "an enemy at an UNSEEN cell is NOT visible"
    );
}
