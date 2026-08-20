//! The band at an emplacement's cell belongs to whoever stands there, not to the toggle.

use super::support::*;
use crate::{
    cover::HeightBand,
    ganger::StanceKind,
    terrain::emplacement::{EmplacementState, SetEmplacement},
};

#[test]
fn the_occupant_band_follows_the_ganger_on_and_off_the_seat() {
    let at = key(4, 4, 0);
    let start = key(2, 4, 0);
    let mut app = headless_app();
    let emplacement = spawn_vacant_emplacement(&mut app, at);
    let ganger = stanced_ganger_at(&mut app, StanceKind::Crouching, start);
    app.update();

    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Vacant),
        "an emplacement spawns Vacant (the default)",
    );
    assert_eq!(
        occupant_band(&app, at),
        None,
        "an unmanned emplacement publishes no occupant band",
    );

    toggle_and_settle(&mut app, SetEmplacement::occupy(emplacement, ganger));
    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Occupied),
        "occupy flips the emplacement to Occupied",
    );
    assert_eq!(
        occupant(&app, emplacement),
        Some(ganger),
        "occupy records the manning ganger as the occupant",
    );
    assert_eq!(
        occupant_band(&app, at),
        Some(HeightBand::Mid),
        "the seat's band is the occupant's own CROUCHING silhouette (MID), published because \
         the occupant now stands there — nothing forces a band on the seat",
    );

    toggle_and_settle(&mut app, SetEmplacement::vacate(emplacement, ganger));
    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Vacant),
        "vacate flips the emplacement back to Vacant",
    );
    assert_eq!(
        occupant(&app, emplacement),
        None,
        "vacate removes the occupant record",
    );
    assert_eq!(
        occupant_band(&app, at),
        None,
        "the occupant walked off the seat, so its band went with it and the seat publishes none",
    );
    assert_eq!(
        occupant_band(&app, start),
        Some(HeightBand::Mid),
        "the band is back on the cell the ganger entered from, still its CROUCHING silhouette",
    );
}

#[test]
fn vacate_restores_standing_band() {
    let at = key(7, 2, 0);
    let start = key(9, 2, 0);
    let mut app = headless_app();
    let emplacement = spawn_vacant_emplacement(&mut app, at);
    let ganger = stanced_ganger_at(&mut app, StanceKind::Standing, start);
    app.update();

    toggle_and_settle(&mut app, SetEmplacement::occupy(emplacement, ganger));
    toggle_and_settle(&mut app, SetEmplacement::vacate(emplacement, ganger));
    assert_eq!(
        occupant_band(&app, start),
        Some(HeightBand::High),
        "a standing occupant's band is back on its entry cell after the vacate",
    );
}

#[test]
fn occupy_on_occupied_is_rejected_no_force_eject() {
    let at = key(5, 5, 0);
    let first_start = key(3, 5, 0);
    let second_start = key(7, 5, 0);
    let mut app = headless_app();
    let emplacement = spawn_vacant_emplacement(&mut app, at);
    let first = stanced_ganger_at(&mut app, StanceKind::Standing, first_start);
    let second = stanced_ganger_at(&mut app, StanceKind::Prone, second_start);
    app.update();

    toggle_and_settle(&mut app, SetEmplacement::occupy(emplacement, first));
    assert_eq!(
        occupant(&app, emplacement),
        Some(first),
        "the first ganger mans the emplacement",
    );

    toggle_and_settle(&mut app, SetEmplacement::occupy(emplacement, second));
    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Occupied),
        "re-occupying an Occupied emplacement stays Occupied",
    );
    assert_eq!(
        occupant(&app, emplacement),
        Some(first),
        "re-occupying does NOT displace the seated occupant (no force-eject)",
    );
    assert_eq!(
        position_of(&app, second),
        Some(second_start),
        "the refused ganger never left its own cell (no force-eject)",
    );
    assert_eq!(
        occupant_band(&app, at),
        Some(HeightBand::High),
        "the seat's HIGH band traces to the seated STANDING occupant alone; the rejected \
         re-occupy moved nobody onto it",
    );
}

#[test]
fn a_ganger_with_no_position_mans_the_seat_and_publishes_no_band() {
    let at = key(6, 6, 0);
    let mut app = headless_app();
    let emplacement = spawn_vacant_emplacement(&mut app, at);
    let ganger = stanced_ganger(&mut app, StanceKind::Standing);
    app.update();

    toggle_and_settle(&mut app, SetEmplacement::occupy(emplacement, ganger));

    assert_eq!(
        occupant_band(&app, at),
        None,
        "the occupant carries no Position, so nobody stands on the seat and no band is \
         published there — the toggle writes none of its own",
    );
    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Occupied),
        "a ganger the position write cannot reach still mans the emplacement",
    );
    assert_eq!(
        occupant(&app, emplacement),
        Some(ganger),
        "a ganger the position write cannot reach is still recorded as the occupant",
    );
    assert_eq!(
        entered_from(&app, emplacement),
        None,
        "with no Position to read there is no entry cell to remember, and no default is \
         substituted for one",
    );
}
