use gdtf_battle_sim::{
    acts::{EnterEmplacementRequested, ExitEmplacementRequested},
    cover::HeightBand,
    ganger::Direction,
    terrain::emplacement::EmplacementState,
    test_support::SituationBuilder,
};

use super::harness::*;


#[test]
fn enter_mans_and_spawns_mount_exit_reverts_and_despawns_mount() {
    let (mut app, seed) = battle_app(0x5543_0A0A);
    let situation = SituationBuilder::new()
        .with_gangers([player_at(ground(5, 5), Direction::East)])
        .build_with_gangs();
    drive_setup(&mut app, seed, situation);

    let emp_cell = ground(6, 5);
    let emplacement = spawn_emplacement(&mut app, emp_cell);
    let Some(actor) = player_ganger(&mut app) else {
        unreachable!("setup spawns one player ganger");
    };
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
        occupant_band(&app, emp_cell),
        Some(HeightBand::High),
        "ENTER: the occupant band is forced HIGH (reads as HIGH cover)",
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
        occupant_band(&app, emp_cell),
        Some(HeightBand::High),
        "EXIT: the band is restored from the STANDING occupant's stance silhouette (HIGH)",
    );
    assert_eq!(
        tu_of(&app, actor),
        Some(tu_after_enter - exit_cost),
        "EXIT: the actor's TU dropped by EXACTLY the exit_emplacement_tu leaf",
    );
}
