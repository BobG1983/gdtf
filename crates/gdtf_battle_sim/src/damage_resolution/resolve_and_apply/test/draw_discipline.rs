use super::support::*;
use crate::rng::InjuryRng;

fn fresh_injury_rng() -> InjuryRng {
    injury_rng()
}

fn severity_of(report: &HitReport) -> Option<Severity> {
    applied_of(report).map(|applied| applied.severity)
}

fn a_biased_weapon(damage: i32, fatal_bias: f32) -> WeaponBundle {
    let spec = FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(1.0),
        ModeShots::new(1),
    );
    WeaponBundle::new(
        WeaponName::new("test-biased-weapon".to_owned()),
        BaseSpread::new(0.1),
        Accuracy::new(1.0),
        Kickback::new(0.0),
        FatalBias::new(fatal_bias),
        DamageProfile::new(
            WeaponDamage::new(damage),
            WeaponPunch::new(0),
            WeaponShred::new(0),
            DamageType::Kinetic,
        ),
        HandlingProfile::new(
            Magazine::loaded(MagazineSize::new(10), ReloadTu::new(10)),
            FireMode::new(vec![spec]),
            Stable::new(false),
            Shove::new(false),
            Handedness::OneHanded,
        ),
    )
}

fn assert_one_severity_draw(mut used: SeverityRng, what: &str) {
    let mut reference = rng();
    let _term: f32 = reference.random_range(0.0f32..1.0f32);
    assert_eq!(
        used.next_u64(),
        reference.next_u64(),
        "{what} must take EXACTLY ONE SeverityRng draw",
    );
}

fn assert_no_severity_draw(mut used: SeverityRng, what: &str) {
    assert_eq!(
        used.next_u64(),
        rng().next_u64(),
        "{what} must take NO SeverityRng draw",
    );
}

fn assert_one_injury_draw(mut used: InjuryRng, what: &str) {
    let mut reference = fresh_injury_rng();
    let _pick: u64 = reference.random_range(0..=0u64);
    assert_eq!(
        used.next_u64(),
        reference.next_u64(),
        "{what} must take EXACTLY ONE InjuryRng draw (empty-table draw-then-discard)",
    );
}

fn assert_no_injury_draw(mut used: InjuryRng, what: &str) {
    assert_eq!(
        used.next_u64(),
        fresh_injury_rng().next_u64(),
        "{what} must take NO InjuryRng draw",
    );
}

fn fold_live_ganger(
    weapon: &WeaponBundle,
    toughness: Toughness,
) -> (HitReport, SeverityRng, InjuryRng) {
    let tuning = CombatTuning::default();
    let entity = an_entity();
    let outcome = ganger_outcome(entity, BodyPart::Torso);

    let mut hp = Hp::new(10_000);
    let mut wounds = Wounds::new(200);
    let mut life = LifeState::Alive;
    let mut inflicted = InflictedWounds::default();

    let mut sev = rng();
    let mut inj = fresh_injury_rng();
    let report = resolve_and_apply(
        &outcome,
        weapon.stats(),
        Luck::new(0.0),
        Some(TargetGanger {
            hp: &mut hp,
            wounds: &mut wounds,
            life: &mut life,
            piece: None, 
            inflicted: &mut inflicted,
            toughness,
            luck: Luck::new(0.0),
        }),
        entity,
        surfaces(&mut ledger(), &mut slab_ledger()),
        &tuning,
        &mut sev,
        &injury_tables(),
        &injury_registry(),
        &mut inj,
    );
    (report, sev, inj)
}

#[test]
fn wound_draws_are_severity_gated_one_severity_and_at_most_one_injury() {
    let mut saw_tabled = false;
    for damage in 1..=40 {
        let weapon = a_weapon(damage, damage / 2, 2, DamageType::Kinetic);
        let (report, sev, inj) = fold_live_ganger(&weapon, Toughness::new(20.0));

        assert_one_severity_draw(sev, "a live ganger wound");

        let severity = severity_of(&report);
        match severity {
            Some(Severity::Minor | Severity::Major | Severity::Critical) => {
                saw_tabled = true;
                assert!(
                    ganger_verdict(&report).is_some_and(|v| v.injury.is_none()),
                    "an empty injury table must freeze NO injury (the draw is discarded)",
                );
                assert_one_injury_draw(inj, "a tabled ganger wound (empty table)");
            }
            Some(Severity::None | Severity::Fatal) => {
                assert_no_injury_draw(inj, "a graze / fatal wound");
            }
            None => {
                assert!(severity.is_some(), "a live ganger hit must apply a wound");
            }
        }
    }
    assert!(
        saw_tabled,
        "the damage ladder must land at least one tabled (Minor/Major/Critical) wound \
         under the fixed SEED — otherwise the exactly-one-injury-draw pin asserts nothing",
    );
}

#[test]
fn graze_takes_one_severity_draw_and_no_injury_draw() {
    let weapon = a_biased_weapon(10, -1.0e6);
    let (report, sev, inj) = fold_live_ganger(&weapon, Toughness::new(0.0));

    assert_eq!(
        severity_of(&report),
        Some(Severity::None),
        "fixture: the huge negative fatal bias must force a graze",
    );

    assert_one_severity_draw(sev, "a graze");
    assert_no_injury_draw(inj, "a graze");
}

#[test]
fn fatal_takes_one_severity_draw_and_no_injury_draw() {
    let weapon = a_biased_weapon(50, 1.0e6);
    let (report, sev, inj) = fold_live_ganger(&weapon, Toughness::new(0.0));

    assert_eq!(
        severity_of(&report),
        Some(Severity::Fatal),
        "fixture: the huge positive fatal bias must force a Fatal",
    );

    assert_one_severity_draw(sev, "a fatal wound");
    assert_no_injury_draw(inj, "a fatal wound");
}

#[test]
fn corpse_skip_takes_neither_draw() {
    let tuning = CombatTuning::default();
    let entity = an_entity();
    let weapon = a_weapon(40, 30, 10, DamageType::Kinetic);
    let outcome = ganger_outcome(entity, BodyPart::Torso);

    let mut hp = Hp::new(15);
    let mut wounds = Wounds::new(3);
    let mut life = LifeState::Dead; 
    let mut inflicted = InflictedWounds::default();

    let mut sev = rng();
    let mut inj = fresh_injury_rng();
    let _report = resolve_and_apply(
        &outcome,
        weapon.stats(),
        Luck::new(0.0),
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
        &tuning,
        &mut sev,
        &injury_tables(),
        &injury_registry(),
        &mut inj,
    );

    assert_no_severity_draw(sev, "a corpse-skip");
    assert_no_injury_draw(inj, "a corpse-skip");
}

#[test]
fn defensive_no_part_takes_neither_draw() {
    let tuning = CombatTuning::default();
    let entity = an_entity();
    let weapon = a_weapon(40, 30, 10, DamageType::Kinetic);
    let outcome = ShotOutcome {
        body_part: None,
        ..ganger_outcome(entity, BodyPart::Torso)
    };

    let mut hp = Hp::new(50);
    let mut wounds = Wounds::new(9);
    let mut life = LifeState::Alive;
    let mut inflicted = InflictedWounds::default();

    let mut sev = rng();
    let mut inj = fresh_injury_rng();
    let report = resolve_and_apply(
        &outcome,
        weapon.stats(),
        Luck::new(0.0),
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
        &tuning,
        &mut sev,
        &injury_tables(),
        &injury_registry(),
        &mut inj,
    );

    assert_eq!(
        report.verdict,
        HitVerdict::NoEffect,
        "a no-part ganger outcome must fold to no effect",
    );
    assert_no_severity_draw(sev, "a defensive no-part fold");
    assert_no_injury_draw(inj, "a defensive no-part fold");
}

#[test]
fn structural_and_miss_arms_take_zero_draws_on_both_streams() {
    let tuning = CombatTuning::default();
    let weapon = a_weapon(40, 20, 10, DamageType::Kinetic);

    let outcomes = [
        cover_outcome(cover_entry(50, 2, 1)),
        slab_outcome(),
        ground_outcome(),
        non_ganger_outcome(ShotKind::Miss),
    ];
    for outcome in outcomes {
        let mut cover = ledger();
        let mut slab = slab_ledger();
        let mut sev = rng();
        let mut inj = fresh_injury_rng();
        let _report = resolve_and_apply(
            &outcome,
            weapon.stats(),
            Luck::new(0.0),
            None,
            an_entity(),
            surfaces(&mut cover, &mut slab),
            &tuning,
            &mut sev,
            &injury_tables(),
            &injury_registry(),
            &mut inj,
        );
        let what = format!("a {:?} outcome", outcome.kind);
        assert_no_severity_draw(sev, &what);
        assert_no_injury_draw(inj, &what);
    }
}
