//! readback of the authored placement + name.

use super::support::*;

#[test]
fn setup_spawns_exactly_the_authored_ganger_count() {
    let (situation, ..) = minimal_fixture();
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };

    assert_eq!(
        *setup.ganger_count(),
        2,
        "two authored gangers were spawned"
    );

    let world: &mut World = app.world_mut();
    let mut query = world.query::<&Wears>();
    assert_eq!(
        query.iter(world).count(),
        2,
        "exactly two ganger entities exist in the world (each wearing armor via Wears)",
    );
}

#[test]
fn each_spawned_ganger_has_all_required_components() {
    let (situation, ..) = minimal_fixture();
    let Some((mut app, _setup)) = run_setup(situation) else {
        return;
    };

    let world: &mut World = app.world_mut();
    let mut all = world.query::<(
        &Position,
        &GangerName,
        &Faction,
        &Facing,
        &Stance,
        &Aiming,
        (&Hp, &HpMax, &Wounds, &WoundsMax),
        &Tu,
        &TuMax,
        &LifeState,
        &Shooting,
        &Toughness,
        &Luck,
        &Wears,
    )>();
    assert_eq!(
        all.iter(world).count(),
        2,
        "both gangers must carry the full E1.2 set (incl. TuMax + HpMax + WoundsMax + GangerName) \
         + E3.0 attribute stats + the Wears armor relationship",
    );
}

#[test]
fn spawned_position_and_faction_match_the_fixture() {
    let (situation, alice_at, bob_at, ..) = minimal_fixture();
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };

    let placements = &setup.occupants;
    assert_eq!(placements.len(), 2);

    let world: &mut World = app.world_mut();
    let mut q = world.query::<(&Position, &Faction)>();

    let alice: Entity = placements[0].occupant;
    assert_eq!(placements[0].at, alice_at);
    let alice_components = q.get(world, alice);
    assert!(
        alice_components.is_ok(),
        "alice's spawned entity must carry Position + Faction",
    );
    let Ok((alice_pos, alice_faction)) = alice_components else {
        return;
    };
    assert_eq!(
        *alice_pos,
        Position::new(alice_at),
        "alice Position matches"
    );
    assert_eq!(*alice_faction, Faction::new(0), "alice Faction matches");

    let bob: Entity = placements[1].occupant;
    assert_eq!(placements[1].at, bob_at);
    let bob_components = q.get(world, bob);
    assert!(
        bob_components.is_ok(),
        "bob's spawned entity must carry Position + Faction",
    );
    let Ok((bob_pos, bob_faction)) = bob_components else {
        return;
    };
    assert_eq!(*bob_pos, Position::new(bob_at), "bob Position matches");
    assert_eq!(*bob_faction, Faction::new(1), "bob Faction matches");

    assert_ne!(alice, bob, "the two gangers are distinct entities");
}

#[test]
fn setup_seeds_ganger_name_onto_each_ganger() {
    let (situation, ..) = minimal_fixture();
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };

    let alice: Entity = setup.occupants[0].occupant;
    let bob: Entity = setup.occupants[1].occupant;
    let world: &mut World = app.world_mut();
    let mut q = world.query::<&GangerName>();

    let alice_name = q.get(world, alice);
    assert_eq!(
        alice_name.map(|n| (**n).clone()).ok(),
        Some("Ganger 0".to_owned()),
        "alice carries her authored GangerName",
    );

    let bob_name = q.get(world, bob);
    assert_eq!(
        bob_name.map(|n| (**n).clone()).ok(),
        Some("Ganger 1".to_owned()),
        "bob carries his authored GangerName",
    );
}
