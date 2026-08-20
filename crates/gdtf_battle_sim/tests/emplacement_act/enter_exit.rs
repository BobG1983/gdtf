use bevy::{app::App, prelude::Entity};
use gdtf_battle_sim::{
    acts::{EnterEmplacementRequested, ExitEmplacementRequested},
    cover::HeightBand,
    ganger::Direction,
    metric::CellLevel,
    terrain::emplacement::EmplacementState,
    test_support::{SituationBuilder, emplacement_at},
};

use super::harness::*;

/// The cell the actor starts on, and is put back on when it leaves the seat.
fn start() -> CellLevel {
    ground(5, 5)
}

/// The cell the emplacement is seeded on.
fn seat() -> CellLevel {
    ground(6, 5)
}

/// A battle with one player actor on [`start`] and one vacant emplacement on [`seat`].
fn a_seat_beside_the_actor() -> (App, Entity, Entity) {
    let (mut app, seed) = battle_app(0x5543_0A0A);
    let situation = SituationBuilder::new()
        .with_gangers([player_at(start(), Direction::East)])
        .with_scatter(emplacement_at(seat()))
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);
    let emplacement = seated_emplacement(&mut app, seat());
    let Some(actor) = player_ganger(&mut app) else {
        unreachable!("setup spawns one player ganger");
    };
    (app, actor, emplacement)
}

#[test]
fn enter_mans_and_spawns_mount_exit_reverts_and_despawns_mount() {
    let (mut app, actor, emplacement) = a_seat_beside_the_actor();
    let emp_cell = seat();
    let (Some(tu_before), Some(enter_cost)) =
        (tu_of(&app, actor), Some(enter_tu(&app)).filter(|c| *c > 0))
    else {
        unreachable!("the actor carries Tu and the enter leaf is a real positive cost");
    };
    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Vacant),
        "the emplacement starts Vacant",
    );

    app.world_mut()
        .write_message(EnterEmplacementRequested::new(actor, emplacement));
    step(&mut app, 3);

    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Occupied),
        "ENTER: the emplacement is Occupied",
    );
    assert_eq!(
        occupant(&app, emplacement),
        Some(actor),
        "ENTER: the actor is recorded as the occupant",
    );
    assert_eq!(
        tu_of(&app, actor),
        Some(tu_before - enter_cost),
        "ENTER: the actor's TU dropped by EXACTLY the enter_emplacement_tu leaf",
    );
    assert_eq!(
        pos_of(&app, actor),
        Some(emp_cell),
        "ENTER: the actor is standing ON the emplacement's cell, not beside it — the act moved \
         it off {:?}",
        start(),
    );
    assert_eq!(
        (occupant_band(&app, emp_cell), occupant_band(&app, start())),
        (Some(HeightBand::High), None),
        "ENTER: the seat's band is the occupant's OWN standing silhouette, published because it \
         stands there, and the cell it left publishes none",
    );
    assert!(
        mount_entity(&app, emplacement).is_some(),
        "ENTER: the emplacement records the spawned mounted-weapon entity",
    );
    assert!(
        wields_mount(&mut app, actor),
        "ENTER: the occupant wields the MountedWeapon (spawned + related on enter)",
    );

    let tu_after_enter = tu_of(&app, actor).unwrap_or(0);
    let exit_cost = exit_tu(&app);
    assert!(exit_cost > 0, "the exit leaf is a real positive cost");

    app.world_mut()
        .write_message(ExitEmplacementRequested::new(actor, emplacement));
    step(&mut app, 3);

    assert_eq!(
        state(&app, emplacement),
        Some(EmplacementState::Vacant),
        "EXIT: the emplacement is Vacant again",
    );
    assert_eq!(
        occupant(&app, emplacement),
        None,
        "EXIT: the occupant record is cleared",
    );
    assert_eq!(
        mount_entity(&app, emplacement),
        None,
        "EXIT: the mounted-weapon record is cleared",
    );
    assert!(
        !wields_mount(&mut app, actor),
        "EXIT: the occupant no longer wields a MountedWeapon (the mount edge was despawned)",
    );
    assert_eq!(
        pos_of(&app, actor),
        Some(start()),
        "EXIT: the actor is back on the cell it entered from",
    );
    assert_eq!(
        (occupant_band(&app, emp_cell), occupant_band(&app, start())),
        (None, Some(HeightBand::High)),
        "EXIT: the actor walked off the seat, so the seat publishes no band and the STANDING \
         silhouette is back on its entry cell",
    );
    assert_eq!(
        tu_of(&app, actor),
        Some(tu_after_enter - exit_cost),
        "EXIT: the actor's TU dropped by EXACTLY the exit_emplacement_tu leaf",
    );
}
