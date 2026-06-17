//! Spawn-loop tests — entity count, the full component set, per-field readback,
//! attribute-stat seeding, worn-armor seeding, and weapon arming.

use super::support::*;

/// C8(a) — the correct entity COUNT spawned: a 2-ganger fixture spawns exactly
/// two ganger entities (each carrying the worn-armor component) and no more.
#[test]
fn setup_spawns_exactly_the_authored_ganger_count() {
    let (situation, ..) = minimal_fixture();
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };

    assert_eq!(setup.ganger_count(), 2, "two authored gangers were spawned");

    // Exactly two entities carry the per-ganger worn armor — the spawned set.
    let world: &mut World = app.world_mut();
    let mut query = world.query::<&WornArmor>();
    assert_eq!(
        query.iter(world).count(),
        2,
        "exactly two ganger entities exist in the world",
    );
}

/// C8(b) — each spawned ganger carries ALL required components (E1.2 set +
/// `WornArmor`), proven by a full-tuple query matching both entities.
#[test]
fn each_spawned_ganger_has_all_required_components() {
    let (situation, ..) = minimal_fixture();
    let Some((mut app, _setup)) = run_setup(situation) else {
        return;
    };

    let world: &mut World = app.world_mut();
    // A query naming EVERY required component — only an entity carrying all of
    // them matches, so a count of 2 proves both gangers have the full set
    // (E1.2 state + E3.0 attribute stats + WornArmor).
    let mut all = world.query::<(
        &Position,
        &GangerName,
        &Faction,
        &Facing,
        &Stance,
        &Aiming,
        &Hp,
        &Wounds,
        &Tu,
        &TuMax,
        &LifeState,
        &Shooting,
        &Toughness,
        &Luck,
        &WornArmor,
    )>();
    assert_eq!(
        all.iter(world).count(),
        2,
        "both gangers must carry the full E1.2 set (incl. TuMax + GangerName) + E3.0 attribute \
         stats + WornArmor",
    );
}

/// C8(c) — each ganger's Position and Faction match the fixture, looked up by
/// the spawned Entity handle (never a numeric id).
#[test]
fn spawned_position_and_faction_match_the_fixture() {
    let (situation, alice_at, bob_at, ..) = minimal_fixture();
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };

    // The setup returns placements in authored order: alice (faction 0) then
    // bob (faction 1).
    let placements = &setup.occupants;
    assert_eq!(placements.len(), 2);

    let world: &mut World = app.world_mut();
    let mut q = world.query::<(&Position, &Faction)>();

    // Alice — placement 0.
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

    // Bob — placement 1.
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

    // The two are distinct Entity handles.
    assert_ne!(alice, bob, "the two gangers are distinct entities");
}

/// GTW-285 — `setup_battle` seeds each authored `GangerSpawn.name` onto the spawned
/// ganger as a queryable `GangerName` component, looked up by the spawned `Entity`
/// handle (never a numeric id). Reads BOTH gangers — distinct authored names ("Ganger 0"
/// / "Ganger 1") prove the per-ganger seed, not a shared default. Pin-discriminating:
/// dropping the `name` spawn in `setup_battle` leaves no `GangerName` and this fails.
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

    // Alice — faction 0 → the fixture's "Ganger 0" name.
    let alice_name = q.get(world, alice);
    assert_eq!(
        alice_name.map(|n| (**n).clone()).ok(),
        Some("Ganger 0".to_owned()),
        "alice carries her authored GangerName",
    );

    // Bob — faction 1 → the fixture's "Ganger 1" name (a distinct per-ganger seed).
    let bob_name = q.get(world, bob);
    assert_eq!(
        bob_name.map(|n| (**n).clone()).ok(),
        Some("Ganger 1".to_owned()),
        "bob carries his authored GangerName",
    );
}

/// GTW-279 AC2 — `setup_battle` seeds an EMPTY `InflictedWounds` record onto every
/// spawned ganger (alongside the existing vitals), queryable off the spawned `Entity`
/// handle. Reads BOTH gangers — each starts with no recorded wounds. Pin-discriminating:
/// dropping the `InflictedWounds::default()` seed leaves no component and `get` errs.
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
            "{who} must carry a seeded InflictedWounds component (GTW-279 AC2)",
        );
        assert!(
            record.is_ok_and(|r| r.is_empty()),
            "{who}'s InflictedWounds must be seeded EMPTY (no wounds until inflicted)",
        );
    }
}

/// GTW-182 AC #2 + AC #3 — `setup_battle` seeds the E3.0 attribute stats
/// (`Shooting`/`Toughness`/`Luck`) onto each spawned ganger from the authored
/// `GangerSpawn`, and they are queryable off the entity by its spawned `Entity`
/// handle (the read shape the severity roll uses). Reads BOTH gangers — a shooter
/// (faction 0) and a defender (faction 1) — proving the per-ganger seed, not a
/// shared default. Magnitudes match the fixture (`faction + {2,3,1}`), per-ganger
/// data, not pinned tuning.
#[test]
fn setup_seeds_attribute_stats_onto_each_ganger() {
    let (situation, ..) = minimal_fixture();
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };

    let alice: Entity = setup.occupants[0].occupant;
    let bob: Entity = setup.occupants[1].occupant;
    let world: &mut World = app.world_mut();
    // The severity-roll read shape: a tuple query over the three attribute stats.
    let mut q = world.query::<(&Shooting, &Toughness, &Luck)>();

    // Alice — faction 0 → Shooting 2.0 / Toughness 3.0 / Luck 1.0 (the fixture).
    let alice_stats = q.get(world, alice);
    assert_eq!(
        alice_stats,
        Ok((&Shooting::new(2.0), &Toughness::new(3.0), &Luck::new(1.0))),
        "alice carries her authored Shooting/Toughness/Luck",
    );

    // Bob — faction 1 → Shooting 3.0 / Toughness 4.0 / Luck 2.0 (distinct seed).
    let bob_stats = q.get(world, bob);
    assert_eq!(
        bob_stats,
        Ok((&Shooting::new(3.0), &Toughness::new(4.0), &Luck::new(2.0))),
        "bob carries his authored Shooting/Toughness/Luck",
    );
}

/// C8(d) — the worn armor on a spawned ganger equals the fixture's roster armor,
/// field-by-field across all six parts (E1.3 seed-from on the real spawn path).
#[test]
fn spawned_worn_armor_matches_the_fixture_roster() {
    let (situation, ..) = minimal_fixture();
    // The fixture's alice roster armor (faction 0 → base 1).
    let expected = arbitrary_armor(1);
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };

    let alice: Entity = setup.occupants[0].occupant;
    let world: &mut World = app.world_mut();
    let mut q = world.query::<&WornArmor>();
    let worn_result = q.get(world, alice);
    assert!(
        worn_result.is_ok(),
        "alice must carry a WornArmor component",
    );
    let Ok(worn) = worn_result else {
        return;
    };
    for part in BodyPart::ALL {
        assert_eq!(
            worn.at(part),
            expected.at(part),
            "worn armor at {part:?} must equal the seeded roster piece",
        );
    }
}

/// GTW-257 AC2 — `setup_battle` ARMS each ganger from the registry: every spawned
/// ganger entity carries the [`Weapon`] marker + its [`WeaponName`] (= the authored
/// key) + the [`FireMode`] selector. Mirrors the worn-armor assertion (the
/// `each_spawned_ganger_has_all_required_components` precedent), proving the resolved
/// [`WeaponBundle`] landed on the real spawn path.
#[test]
fn setup_arms_each_ganger_from_the_registry() {
    let (situation, ..) = minimal_fixture();
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };

    let world: &mut World = app.world_mut();
    // A query naming the weapon marker + the name + the fire-mode selector — only an
    // armed entity matches, so a count of 2 proves BOTH gangers were armed.
    let mut armed = world.query::<(&Weapon, &WeaponName, &FireMode)>();
    assert_eq!(
        armed.iter(world).count(),
        2,
        "both gangers must carry the Weapon marker + WeaponName + FireMode (armed from the \
         registry)",
    );

    // The first ganger's WeaponName is the authored key, looked up by its spawned
    // Entity handle (never a numeric id).
    let alice: Entity = setup.occupants[0].occupant;
    let mut name_q = world.query::<&WeaponName>();
    let alice_name = name_q.get(world, alice);
    assert_eq!(
        alice_name.map(|n| (**n).clone()).ok(),
        Some(TEST_WEAPON_KEY.to_owned()),
        "the armed ganger's WeaponName equals the authored weapon key",
    );
}
