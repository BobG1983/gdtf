//! Spawn-loop tests — entity count, the full component set, and per-field
//! readback of the authored placement + name.

use super::support::*;

/// C8(a) — the correct entity COUNT spawned: a 2-ganger fixture spawns exactly
/// two ganger entities (each carrying the `Wears` armor relationship) and no more.
#[test]
fn setup_spawns_exactly_the_authored_ganger_count() {
    let (situation, ..) = minimal_fixture();
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };

    assert_eq!(setup.ganger_count(), 2, "two authored gangers were spawned");

    // Exactly two gangers carry the per-ganger `Wears` armor relationship — the spawned
    // set. Since GTW-323 slice 3 (ADR-0004) the armor stats live on related piece
    // entities, NOT a `WornArmor` component on the ganger, so a ganger is identified by
    // its `Wears` collection.
    let world: &mut World = app.world_mut();
    let mut query = world.query::<&Wears>();
    assert_eq!(
        query.iter(world).count(),
        2,
        "exactly two ganger entities exist in the world (each wearing armor via Wears)",
    );
}

/// C8(b) — each spawned ganger carries ALL required components (E1.2 set + the `Wears`
/// armor relationship), proven by a full-tuple query matching both entities.
#[test]
fn each_spawned_ganger_has_all_required_components() {
    let (situation, ..) = minimal_fixture();
    let Some((mut app, _setup)) = run_setup(situation) else {
        return;
    };

    let world: &mut World = app.world_mut();
    // A query naming EVERY required component — only an entity carrying all of
    // them matches, so a count of 2 proves both gangers have the full set
    // (E1.2 state + E3.0 attribute stats + the `Wears` armor relationship — the armor
    // STATS themselves live on the related piece entities, GTW-323 slice 3, NOT here).
    // The vitals pools + their display ceilings are grouped into a nested sub-tuple:
    // Bevy's `QueryData` tuple impls cap at 16 elements, and the full set now numbers
    // 17 (GTW-291 added `HpMax` / `WoundsMax`), so nesting keeps the outer arity legal
    // while still requiring every component to match.
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
