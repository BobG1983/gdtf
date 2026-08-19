use super::support::*;
use crate::{
    resolve_and_apply::{ShotSource, WoundRoll},
    rng::{BattleSeed, InjuryRng},
};

fn fresh_injury_rng() -> InjuryRng {
    InjuryRng::from_root(BattleSeed::new(SEED))
}

fn assert_no_injury_draw(mut rng: InjuryRng, what: &str) {
    assert_eq!(
        rng.next_u64(),
        fresh_injury_rng().next_u64(),
        "{what} must take NO InjuryRng draw (the cursor must be unadvanced)",
    );
}

#[test]
fn corpse_hit_takes_no_injury_draw() {
    let tuning = CombatTuning::default();
    let entity = an_entity();
    let weapon = a_weapon(20, 12, 8, DamageType::Plasma);
    let outcome = ganger_outcome(entity, BodyPart::Head);

    let mut hp = Hp::new(15);
    let mut wounds = Wounds::new(3);
    let mut life = LifeState::Dead;
    let mut integrity = piece_integrity(1);
    let mut inflicted = InflictedWounds::default();

    let mut inj = fresh_injury_rng();
    let _report = resolve_and_apply(
        &outcome,
        ShotSource {
            weapon: weapon.stats(),
            luck:   Luck::new(0.0),
        },
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
        &mut WoundRoll {
            tuning:       &tuning,
            severity_rng: &mut rng(),
            tables:       &injury_tables(),
            registry:     &injury_registry(),
            injury_rng:   &mut inj,
        },
    );
    assert_no_injury_draw(inj, "a corpse-hit");
}

#[test]
fn cover_hit_takes_no_injury_draw() {
    let tuning = CombatTuning::default();
    let weapon = a_weapon(40, 20, 10, DamageType::Kinetic);
    let entry = cover_entry(5, 0, 0, TerrainPieceKind::Cover);
    let outcome = cover_outcome(entry);
    let mut cover = ledger();
    let mut slab = slab_ledger();

    let mut inj = fresh_injury_rng();
    let _report = resolve_and_apply(
        &outcome,
        ShotSource {
            weapon: weapon.stats(),
            luck:   Luck::new(0.0),
        },
        None,
        an_entity(),
        surfaces(&mut cover, &mut slab),
        &mut WoundRoll {
            tuning:       &tuning,
            severity_rng: &mut rng(),
            tables:       &injury_tables(),
            registry:     &injury_registry(),
            injury_rng:   &mut inj,
        },
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
        ShotSource {
            weapon: weapon.stats(),
            luck:   Luck::new(0.0),
        },
        None,
        an_entity(),
        surfaces(&mut cover, &mut slab),
        &mut WoundRoll {
            tuning:       &tuning,
            severity_rng: &mut rng(),
            tables:       &injury_tables(),
            registry:     &injury_registry(),
            injury_rng:   &mut inj,
        },
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
        ShotSource {
            weapon: weapon.stats(),
            luck:   Luck::new(0.0),
        },
        None,
        an_entity(),
        surfaces(&mut cover, &mut slab),
        &mut WoundRoll {
            tuning:       &tuning,
            severity_rng: &mut rng(),
            tables:       &injury_tables(),
            registry:     &injury_registry(),
            injury_rng:   &mut inj,
        },
    );
    assert_no_injury_draw(inj, "a ground-hit");
}

#[test]
fn ganger_wound_takes_one_injury_draw() {
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
        ShotSource {
            weapon: weapon.stats(),
            luck:   Luck::new(0.0),
        },
        Some(TargetGanger {
            hp:        &mut hp,
            wounds:    &mut wounds,
            life:      &mut life,
            piece:     None,
            inflicted: &mut inflicted,
            toughness: Toughness::new(0.0),
            luck:      Luck::new(0.0),
        }),
        entity,
        surfaces(&mut ledger(), &mut slab_ledger()),
        &mut WoundRoll {
            tuning:       &tuning,
            severity_rng: &mut rng(),
            tables:       &injury_tables(),
            registry:     &injury_registry(),
            injury_rng:   &mut inj,
        },
    );

    let severity = applied_of(&report).map(|a| a.severity);
    let is_tabled = matches!(
        severity,
        Some(Severity::Minor | Severity::Major | Severity::Critical)
    );
    if !is_tabled {
        return;
    }
    assert_ne!(
        inj.next_u64(),
        fresh_injury_rng().next_u64(),
        "a non-graze, non-fatal ganger wound must take ONE InjuryRng draw (cursor advances)"
    );
}
