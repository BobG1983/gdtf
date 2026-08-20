//! The toggle runs before the grid sync, so an enter reaches the grid in its own frame.

use super::support::*;
use crate::{ganger::StanceKind, terrain::emplacement::SetEmplacement};

#[test]
fn one_update_is_enough_for_the_grid_to_hold_the_occupant_on_the_seat() {
    let at = key(9, 4, 0);
    let start = key(8, 4, 0);
    let mut app = headless_app();
    let emplacement = spawn_vacant_emplacement(&mut app, at);
    let ganger = stanced_ganger_at(&mut app, StanceKind::Standing, start);

    app.world_mut()
        .write_message(SetEmplacement::occupy(emplacement, ganger));
    app.update();

    assert_eq!(
        grid_occupant(&app, at),
        Some(ganger),
        "the toggle writes the occupant's Position and sync_moved_gangers projects it in the \
         SAME frame; ordered the other way the grid would still be empty at {at:?} after one \
         update",
    );
}
