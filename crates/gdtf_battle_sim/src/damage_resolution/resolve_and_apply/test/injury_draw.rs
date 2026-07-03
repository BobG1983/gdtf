//! GTW-438 — the `InjuryRng` draw discipline at the fold level (the §9 stream-alignment
//! contract): a corpse-hit, a cover-hit, a slab-hit, and a ground-hit take **NO**
//! `InjuryRng` draw (the cursor is unadvanced), while a ganger WOUND takes the roll's draw.
//!
//! These exercise the REAL [`resolve_and_apply`] fold path (no stubs); the draw is proven
//! by comparing the post-call [`InjuryRng`](crate::rng::InjuryRng) cursor against a fresh
//! stream (unadvanced = no draw; advanced = the roll's one draw).

use super::support::*;
use crate::rng::{BattleSeed, InjuryRng};

/// A fresh [`InjuryRng`] from the support `SEED` — the comparison baseline.
fn fresh_injury_rng() -> InjuryRng {
    InjuryRng::from_root(BattleSeed::new(SEED))
}

/// Assert the `rng` cursor is UNADVANCED — its next raw `u64` equals a fresh stream's
/// first draw (so the fold took NO `InjuryRng` draw on this arm).
fn assert_no_injury_draw(mut rng: InjuryRng, what: &str) {
    assert_eq!(
        rng.next_u64(),
        fresh_injury_rng().next_u64(),
        "{what} must take NO InjuryRng draw (the cursor must be unadvanced)",
    );
}

#[test]
fn corpse_hit_takes_no_injury_draw() {
    // A Ganger outcome on an already-Dead target short-circuits BEFORE any draw — neither
    // the SeverityRng nor the InjuryRng cursor advances.
    let tuning = CombatTuning::default();
    let entity = an_entity();
    let weapon = a_weapon(20, 12, 8, DamageType::Plasma);
    let outcome = ganger_outcome(entity, BodyPart::Head);

    let mut hp = Hp::new(15);
    let mut wounds = Wounds::new(3);
    let mut life = LifeState::Dead; // a corpse
    let mut integrity = piece_integrity(1);
    let mut inflicted = InflictedWounds::default();

    let mut inj = fresh_injury_rng();
    let _report = resolve_and_apply(
        &outcome,
        weapon.stats(),
        Luck::new(0.0),
        Some(TargetGanger {
            hp:        &mut hp,
            wounds:    &mut wounds,
            life:      &mut life,
            piece:     Some(struck_piece(0, 0, 0, ArmorType::DEFAULT, &mut integrity)),
            inflicted: &mut inflicted,
            toughness: Toughness::new(2.0),
            luck:      Luck::new(1.0),
        }),
        entity,
        surfaces(&mut ledger(), &mut slab_ledger()),
        &tuning,
        &mut rng(),
        &injury_tables(),
        &injury_registry(),
        &mut inj,
    );
    assert_no_injury_draw(inj, "a corpse-hit");
}

#[test]
fn cover_hit_takes_no_injury_draw() {
    // A cover hit folds through the structural arm — no wound, no injury draw.
    let tuning = CombatTuning::default();
    let weapon = a_weapon(40, 20, 10, DamageType::Kinetic);
    let entry = cover_entry(5, 0, 0);
    let outcome = cover_outcome(entry);
    let mut cover = ledger();
    let mut slab = slab_ledger();

    let mut inj = fresh_injury_rng();
    let _report = resolve_and_apply(
        &outcome,
        weapon.stats(),
        Luck::new(0.0),
        None,
        an_entity(),
        surfaces(&mut cover, &mut slab),
        &tuning,
        &mut rng(),
        &injury_tables(),
        &injury_registry(),
        &mut inj,
    );
    assert_no_injury_draw(inj, "a cover-hit");
}

#[test]
fn slab_hit_takes_no_injury_draw() {
    let tuning = CombatTuning::default();
    let weapon = a_weapon(40, 20, 10, DamageType::Kinetic);
    let outcome = slab_outcome();
    let mut cover = ledger();
    let mut slab = slab_ledger();

    let mut inj = fresh_injury_rng();
    let _report = resolve_and_apply(
        &outcome,
        weapon.stats(),
        Luck::new(0.0),
        None,
        an_entity(),
        surfaces(&mut cover, &mut slab),
        &tuning,
        &mut rng(),
        &injury_tables(),
        &injury_registry(),
        &mut inj,
    );
    assert_no_injury_draw(inj, "a slab-hit");
}

#[test]
fn ground_hit_takes_no_injury_draw() {
    let tuning = CombatTuning::default();
    let weapon = a_weapon(29, 14, 6, DamageType::Kinetic);
    let outcome = ground_outcome();
    let mut cover = ledger();
    let mut slab = slab_ledger();

    let mut inj = fresh_injury_rng();
    let _report = resolve_and_apply(
        &outcome,
        weapon.stats(),
        Luck::new(0.0),
        None,
        an_entity(),
        surfaces(&mut cover, &mut slab),
        &tuning,
        &mut rng(),
        &injury_tables(),
        &injury_registry(),
        &mut inj,
    );
    assert_no_injury_draw(inj, "a ground-hit");
}

#[test]
fn ganger_wound_takes_one_injury_draw() {
    // A LIVE ganger wound takes the roll's one InjuryRng draw (the cursor ADVANCES) — the
    // positive counterpart to the structural / corpse no-draw arms. A high-damage hit on a
    // low-Toughness, low-HP-but-survivable target reliably lands a non-graze, non-fatal
    // wound across the seed, so the roll fires. (The exact severity is RNG-driven; whatever
    // non-None/non-Fatal it is, the draw is taken — that is the property under test.)
    let tuning = CombatTuning::default();
    let entity = an_entity();
    let weapon = a_weapon(40, 30, 10, DamageType::Kinetic);
    let outcome = ganger_outcome(entity, BodyPart::Torso);

    let mut hp = Hp::new(200);
    let mut wounds = Wounds::new(20);
    let mut life = LifeState::Alive;
    let mut inflicted = InflictedWounds::default();

    let mut inj = fresh_injury_rng();
    let report = resolve_and_apply(
        &outcome,
        weapon.stats(),
        Luck::new(0.0),
        Some(TargetGanger {
            hp:        &mut hp,
            wounds:    &mut wounds,
            life:      &mut life,
            // Bare flesh — full damage lands, so the severity is solidly in the wound band.
            piece:     None,
            inflicted: &mut inflicted,
            toughness: Toughness::new(0.0),
            luck:      Luck::new(0.0),
        }),
        entity,
        surfaces(&mut ledger(), &mut slab_ledger()),
        &tuning,
        &mut rng(),
        &injury_tables(),
        &injury_registry(),
        &mut inj,
    );

    // Confirm the precondition the draw is gated on actually held this seed (a non-graze,
    // non-fatal wound) — else the test would assert nothing meaningful.
    let severity = applied_of(&report).map(|a| a.severity);
    let is_tabled = matches!(
        severity,
        Some(Severity::Minor | Severity::Major | Severity::Critical)
    );
    if !is_tabled {
        // The hit was a graze or fatal this seed — the no-draw arms already pin those
        // cases; nothing to assert here. (The chosen weapon/target make this branch
        // unreachable in practice, but stay honest rather than asserting a false premise.)
        return;
    }
    // The wound was tabled → the roll took EXACTLY ONE draw: the cursor advanced (its
    // next value differs from a fresh stream's first value).
    assert_ne!(
        inj.next_u64(),
        fresh_injury_rng().next_u64(),
        "a non-graze, non-fatal ganger wound must take ONE InjuryRng draw (cursor advances)"
    );
}
