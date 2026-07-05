//! Derived-stat / caps / wound-record seeding (GTW-279 / GTW-291 / GTW-384).

use super::support::*;

/// GTW-291 / GTW-384 — `setup_battle` seeds each ganger's `HpMax` / `WoundsMax` as the
/// DERIVED full capacity (no longer authored): `HpMax == derived Hp` and
/// `WoundsMax == derived Wounds` (full at battle start, the current-pool == max
/// contract). Asserted as the RELATION via `derive_stats` over the fixture's authored
/// attributes × the default `GangerStatTuning` — NOT a pinned shipped magnitude.
/// Pin-discriminating: dropping the `hp_max` / `wounds_max` spawn leaves no component
/// and this fails; a max that does not equal the derived pool also fails. These are
/// DISPLAY ceilings, not reset targets — read at setup, never after a round flip.
#[test]
fn setup_seeds_hp_max_and_wounds_max_onto_each_ganger() {
    let (situation, ..) = minimal_fixture();
    // The expected derived caps per ganger — RELATION via the single source of truth,
    // over the fixture's authored attributes and the default tuning the setup uses.
    let tuning = GangerStatTuning::default();
    let alice_derived = derive_stats(&attributes_of(&situation.gangers[0]), &tuning);
    let bob_derived = derive_stats(&attributes_of(&situation.gangers[1]), &tuning);
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };

    let alice: Entity = setup.occupants[0].occupant;
    let bob: Entity = setup.occupants[1].occupant;
    let world: &mut World = app.world_mut();
    let mut q = world.query::<(&HpMax, &WoundsMax)>();

    // Alice — HpMax == derived Hp, WoundsMax == derived Wounds (full at battle start).
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

    // Bob — the same relation against HIS authored attributes (a distinct per-ganger derive).
    let bob_caps = q.get(world, bob);
    assert_eq!(
        bob_caps.map(|(h, w)| (*h, *w)).ok(),
        Some((bob_derived.hp_max, bob_derived.wounds_max)),
        "bob's HpMax/WoundsMax equal his DERIVED Hp/Wounds (full at start)",
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

/// GTW-384 (C8(b)) — `setup_battle` seeds each ganger's EIGHT authored DIRECT
/// ATTRIBUTES onto the entity (the raw potential, unchanged from the situation), AND the
/// DERIVED computed stats (`Shooting` here) match the `derive_stats` RELATION over those
/// attributes × the default tuning. Reads BOTH gangers — a shooter (faction 0) and a
/// defender (faction 1) — proving the per-ganger seed + per-ganger derive, not a shared
/// default. The authored attributes (`Toughness` / `Luck` the severity roll reads) match
/// the fixture (`faction + {3,1}`, per-ganger data); the derived `Shooting` is asserted
/// as the FORMULA, never a pinned magnitude.
#[test]
fn setup_seeds_attribute_stats_onto_each_ganger() {
    let (situation, ..) = minimal_fixture();
    // The expected derived Shooting per ganger — RELATION via the single source of truth.
    let tuning = GangerStatTuning::default();
    let alice_attrs = attributes_of(&situation.gangers[0]);
    let bob_attrs = attributes_of(&situation.gangers[1]);
    let alice_derived = derive_stats(&alice_attrs, &tuning);
    let bob_derived = derive_stats(&bob_attrs, &tuning);
    let Some((mut app, setup)) = run_setup(situation) else {
        return;
    };

    let alice: Entity = setup.occupants[0].occupant;
    let bob: Entity = setup.occupants[1].occupant;
    let world: &mut World = app.world_mut();
    // The authored attributes the severity roll reads (Toughness/Luck) + the derived Shooting.
    let mut q = world.query::<(&Toughness, &Luck, &Shooting)>();

    // Alice — authored Toughness/Luck match the fixture; Shooting == the derived relation.
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

    // Bob — the same relation against HIS authored attributes (a distinct per-ganger derive).
    let bob_stats = q.get(world, bob);
    assert_eq!(
        bob_stats.map(|(t, l, s)| (*t, *l, *s)).ok(),
        Some((bob_attrs.toughness, bob_attrs.luck, bob_derived.shooting)),
        "bob carries his authored Toughness/Luck + his DERIVED Shooting",
    );
}
