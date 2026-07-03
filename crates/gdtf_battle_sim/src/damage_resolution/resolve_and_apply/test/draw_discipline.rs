//! GTW-573 — the **severity-gated draw-discipline pin** for the E3.9 fold, on the REAL
//! [`resolve_and_apply`] path (no stubs): the exact per-arm draw counts on BOTH injected
//! streams, proven by stream-cursor comparison against reference streams advanced by the
//! same-width draws.
//!
//! The contract (the load-bearing seeded-replay discipline):
//!
//! - a real ganger hit takes EXACTLY ONE [`SeverityRng`] draw; the [`InjuryRng`] draw is
//!   gated on the ROLLED severity — `Minor`/`Major`/`Critical` take EXACTLY ONE injury
//!   draw (even on an empty/missing table: draw-then-discard, content-independent stream
//!   alignment), while a graze ([`Severity::None`]) and a `Fatal` take NO injury draw;
//! - the corpse-skip and the defensive no-part path take NEITHER draw;
//! - cover / slab / ground / miss take ZERO draws on both streams.
//!
//! "Exactly one" is pinned by advancing a FRESH reference stream by one draw of the same
//! width the fold takes (`roll_severity`'s single uniform `f32` range draw;
//! `roll_injury`'s single `u64` range draw) and asserting the two cursors then read the
//! same next value — an over- or under-draw on either stream fails loudly.

use super::support::*;
use crate::rng::InjuryRng;

/// A fresh [`InjuryRng`] from the shared `SEED` — the reference-baseline stream.
fn fresh_injury_rng() -> InjuryRng {
    injury_rng()
}

/// The rolled severity a report carries for a LANDED ganger hit, else `None` —
/// the one shape-dependent read this pin test makes (kept in one place).
fn severity_of(report: &HitReport) -> Option<Severity> {
    applied_of(report).map(|applied| applied.severity)
}

/// A weapon whose [`FatalBias`] FORCES the severity regime: a huge negative bias keeps
/// the §6 score below the graze edge for ANY bounded roll (a guaranteed
/// [`Severity::None`]); a huge positive bias keeps it above the top edge (a guaranteed
/// [`Severity::Fatal`]). Bound-independent — it forces the REGIME, never pins a shipped
/// tuning magnitude.
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

/// Assert `used` took EXACTLY ONE severity draw: a fresh reference stream advanced by
/// one uniform `f32` range draw (the width [`roll_severity`]'s single `roll_term` draw
/// consumes — bounds do not change a float draw's stream consumption) must then read
/// the same next value.
fn assert_one_severity_draw(mut used: SeverityRng, what: &str) {
    let mut reference = rng();
    let _term: f32 = reference.random_range(0.0f32..1.0f32);
    assert_eq!(
        used.next_u64(),
        reference.next_u64(),
        "{what} must take EXACTLY ONE SeverityRng draw",
    );
}

/// Assert `used` took NO severity draw (the cursor matches a fresh stream).
fn assert_no_severity_draw(mut used: SeverityRng, what: &str) {
    assert_eq!(
        used.next_u64(),
        rng().next_u64(),
        "{what} must take NO SeverityRng draw",
    );
}

/// Assert `used` took EXACTLY ONE injury draw: a fresh reference stream advanced by one
/// `u64` range draw (the width [`roll_injury`](crate::injuries::roll_injury)'s single
/// pick consumes — an EMPTY bucket draws `0..=0`, the same width as a populated pick)
/// must then read the same next value.
fn assert_one_injury_draw(mut used: InjuryRng, what: &str) {
    let mut reference = fresh_injury_rng();
    let _pick: u64 = reference.random_range(0..=0u64);
    assert_eq!(
        used.next_u64(),
        reference.next_u64(),
        "{what} must take EXACTLY ONE InjuryRng draw (empty-table draw-then-discard)",
    );
}

/// Assert `used` took NO injury draw (the cursor matches a fresh stream).
fn assert_no_injury_draw(mut used: InjuryRng, what: &str) {
    assert_eq!(
        used.next_u64(),
        fresh_injury_rng().next_u64(),
        "{what} must take NO InjuryRng draw",
    );
}

/// Fold one ganger outcome on a LIVE bare-flesh target with the given weapon and
/// defender [`Toughness`], returning the report + the two post-fold streams — the
/// shared live-wound harness.
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
            piece: None, // bare flesh — full damage lands
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

/// EVERY live ganger wound takes EXACTLY ONE severity draw, and the injury draw is
/// gated on the ROLLED tier: a tabled (`Minor`/`Major`/`Critical`) wound takes EXACTLY
/// ONE injury draw — taken-then-discarded on the fixture's EMPTY tables (the report
/// carries NO injury), pinning the content-independent stream-alignment property —
/// while a graze / `Fatal` takes none.
///
/// Sweeps a damage ladder (each rung its own fresh seeded streams) so the pin covers
/// whatever tiers the fixed SEED lands, and HARD-REQUIRES at least one tabled wound in
/// the sweep — a ladder that never tables would pin nothing and must fail loudly.
#[test]
fn wound_draws_are_severity_gated_one_severity_and_at_most_one_injury() {
    let mut saw_tabled = false;
    for damage in 1..=40 {
        let weapon = a_weapon(damage, damage / 2, 2, DamageType::Kinetic);
        // A tough defender spreads the ladder across the tiers (graze → Fatal) under
        // the fixed SEED — an arbitrary fixture value, not a shipped magnitude.
        let (report, sev, inj) = fold_live_ganger(&weapon, Toughness::new(20.0));

        // A live bare-flesh torso hit ALWAYS takes the one severity draw.
        assert_one_severity_draw(sev, "a live ganger wound");

        let severity = severity_of(&report);
        match severity {
            Some(Severity::Minor | Severity::Major | Severity::Critical) => {
                saw_tabled = true;
                // The EMPTY fixture tables mean the taken draw was DISCARDED — no
                // injury freezes onto the verdict, yet the cursor advanced exactly once.
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
                // A live bare-flesh hit always applies — a no-verdict report here is
                // a harness bug, not a legal regime.
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

/// A forced GRAZE ([`Severity::None`]) takes EXACTLY ONE severity draw and NO injury
/// draw — the graze is off the injury stream entirely.
#[test]
fn graze_takes_one_severity_draw_and_no_injury_draw() {
    // A huge NEGATIVE fatal bias keeps the §6 score below the graze edge for any
    // bounded roll — a guaranteed Severity::None, regime-forced (no tuning pin).
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

/// A forced FATAL takes EXACTLY ONE severity draw and NO injury draw — a fatal wound
/// is never tabled (death rides the existing terminal gate).
#[test]
fn fatal_takes_one_severity_draw_and_no_injury_draw() {
    // A huge POSITIVE fatal bias keeps the §6 score above the top edge for any bounded
    // roll — a guaranteed Severity::Fatal, regime-forced (no tuning pin).
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

/// The corpse-skip takes NEITHER draw — a dead target short-circuits BEFORE any stream
/// is touched.
#[test]
fn corpse_skip_takes_neither_draw() {
    let tuning = CombatTuning::default();
    let entity = an_entity();
    let weapon = a_weapon(40, 30, 10, DamageType::Kinetic);
    let outcome = ganger_outcome(entity, BodyPart::Torso);

    let mut hp = Hp::new(15);
    let mut wounds = Wounds::new(3);
    let mut life = LifeState::Dead; // a corpse
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

/// The defensive no-part fold (a `Ganger` outcome carrying `body_part: None`) takes
/// NEITHER draw — it no-effects before the wound core runs.
#[test]
fn defensive_no_part_takes_neither_draw() {
    let tuning = CombatTuning::default();
    let entity = an_entity();
    let weapon = a_weapon(40, 30, 10, DamageType::Kinetic);
    // A Ganger outcome WITHOUT a rolled part — the defensive upstream-miss shape.
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

/// Cover / slab / ground / miss take ZERO draws on BOTH streams — the structural and
/// miss arms are RNG-free (replay-safe).
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
