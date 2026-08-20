//! Where the occupant's cell goes: onto the seat, remembered, and back again.

use super::support::*;
use crate::{
    ganger::StanceKind,
    terrain::emplacement::{EmplacementState, SetEmplacement},
};

#[test]
fn occupy_remembers_the_entry_cell_and_vacate_forgets_it() {
    let at = key(4, 8, 0);
    let start = key(3, 8, 0);
    let mut app = headless_app();
    let emplacement = spawn_vacant_emplacement(&mut app, at);
    let ganger = stanced_ganger_at(&mut app, StanceKind::Standing, start);
    app.update();

    toggle_and_settle(&mut app, SetEmplacement::occupy(emplacement, ganger));
    assert_eq!(
        entered_from(&app, emplacement),
        Some(start),
        "the emplacement remembers the cell the ganger STOOD ON before it entered, not its \
         own seat at {at:?}",
    );

    toggle_and_settle(&mut app, SetEmplacement::vacate(emplacement, ganger));
    assert_eq!(
        entered_from(&app, emplacement),
        None,
        "a vacant emplacement remembers nobody's entry cell",
    );
}

#[test]
fn an_occupy_and_a_vacate_in_one_run_still_return_the_ganger_home() {
    let at = key(8, 8, 0);
    let start = key(7, 8, 0);
    let mut app = headless_app();
    let emplacement = spawn_vacant_emplacement(&mut app, at);
    let ganger = stanced_ganger_at(&mut app, StanceKind::Standing, start);
    app.update();

    app.world_mut()
        .write_message(SetEmplacement::occupy(emplacement, ganger));
    app.world_mut()
        .write_message(SetEmplacement::vacate(emplacement, ganger));
    app.update();

    assert_eq!(
        position_of(&app, ganger),
        Some(start),
        "the EnteredFrom the occupy wrote is a queued command, so the vacate later in the same \
         run has to read the in-run record to hand the ganger back its own cell rather than \
         leaving it on the seat at {at:?}",
    );
    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Vacant),
        "both toggles landed: the seat is Vacant again",
    );
}

#[test]
fn a_vacate_with_nothing_remembered_leaves_the_ganger_where_it_stands() {
    let at = key(6, 2, 0);
    let start = key(2, 7, 0);
    let mut app = headless_app();
    let ganger = stanced_ganger_at(&mut app, StanceKind::Standing, start);
    let emplacement = spawn_occupied_emplacement(&mut app, at, ganger);
    app.update();

    toggle_and_settle(&mut app, SetEmplacement::vacate(emplacement, ganger));

    assert_eq!(
        position_of(&app, ganger),
        Some(start),
        "the seat was spawned manned with no EnteredFrom, so the vacate has no cell to write \
         and must leave the ganger alone rather than defaulting it to the origin",
    );
    assert_eq!(
        occupant_band(&app, at),
        None,
        "nobody ever stood on the seat, so the vacate publishes no band there either",
    );
}
