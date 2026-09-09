use super::support::*;

/// DERIVED full capacity (no longer authored): `HpMax == derived Hp` and
/// contract). Asserted as the RELATION via `derive_stats` over the fixture's authored
#[test]
fn setup_seeds_hp_max_and_wounds_max_onto_each_ganger() {
    let (built, ..) = minimal_fixture();
    let tuning = GangerStatTuning::default();
    let alice_derived = derive_stats(&attributes_of(&built.1[0]), &tuning);
    let bob_derived = derive_stats(&attributes_of(&built.1[1]), &tuning);
    let Some((mut app, setup)) = run_setup(built) else {
        return;
    };

    let alice: Entity = setup.occupants[0].occupant;
    let bob: Entity = setup.occupants[1].occupant;
    let world: &mut World = app.world_mut();
    let mut q = world.query::<(&HpMax, &WoundsMax)>();

    let alice_caps = q.get(world, alice);
    assert_eq!(
        alice_caps.map(|(h, w)| (*h, *w)).ok(),
        Some((alice_derived.hp_max, alice_derived.wounds_max)),
        "alice's HpMax/WoundsMax equal her DERIVED Hp/Wounds (full at start)",
    );
    assert_eq!(
        (alice_derived.hp_max, alice_derived.wounds_max),
        (
            HpMax::new(*alice_derived.hp),
            WoundsMax::new(*alice_derived.wounds)
        ),
        "the derived max equals the derived current pool (current == max at battle start)",
    );

    let bob_caps = q.get(world, bob);
    assert_eq!(
        bob_caps.map(|(h, w)| (*h, *w)).ok(),
        Some((bob_derived.hp_max, bob_derived.wounds_max)),
        "bob's HpMax/WoundsMax equal his DERIVED Hp/Wounds (full at start)",
    );
}

#[test]
fn setup_seeds_empty_inflicted_wounds_onto_each_ganger() {
    let (situation, ..) = minimal_fixture();
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };

    let alice: Entity = setup.occupants[0].occupant;
    let bob: Entity = setup.occupants[1].occupant;
    let world: &mut World = app.world_mut();
    let mut q = world.query::<&InflictedWounds>();

    for (who, entity) in [("alice", alice), ("bob", bob)] {
        let record = q.get(world, entity);
        assert!(
            record.is_ok(),
            "{who} must carry a seeded InflictedWounds component ",
        );
        assert!(
            record.is_ok_and(|r| r.is_empty()),
            "{who}'s InflictedWounds must be seeded EMPTY (no wounds until inflicted)",
        );
    }
}

#[test]
fn setup_seeds_attribute_stats_onto_each_ganger() {
    let (built, ..) = minimal_fixture();
    let tuning = GangerStatTuning::default();
    let alice_attrs = attributes_of(&built.1[0]);
    let bob_attrs = attributes_of(&built.1[1]);
    let alice_derived = derive_stats(&alice_attrs, &tuning);
    let bob_derived = derive_stats(&bob_attrs, &tuning);
    let Some((mut app, setup)) = run_setup(built) else {
        return;
    };

    let alice: Entity = setup.occupants[0].occupant;
    let bob: Entity = setup.occupants[1].occupant;
    let world: &mut World = app.world_mut();
    let mut q = world.query::<(&Toughness, &Luck, &Shooting)>();

    let alice_stats = q.get(world, alice);
    assert_eq!(
        alice_stats.map(|(t, l, s)| (*t, *l, *s)).ok(),
        Some((
            alice_attrs.toughness,
            alice_attrs.luck,
            alice_derived.shooting
        )),
        "alice carries her authored Toughness/Luck + her DERIVED Shooting",
    );

    let bob_stats = q.get(world, bob);
    assert_eq!(
        bob_stats.map(|(t, l, s)| (*t, *l, *s)).ok(),
        Some((bob_attrs.toughness, bob_attrs.luck, bob_derived.shooting)),
        "bob carries his authored Toughness/Luck + his DERIVED Shooting",
    );
}
