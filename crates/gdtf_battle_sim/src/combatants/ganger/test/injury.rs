//! GTW-436 — the injury-aware derivation
//! ([`derive_stats_with_injuries`](crate::ganger::derive_stats_with_injuries)) and the
//! injury-ledger re-derive
//! ([`rederive_stats_on_injury_change`](crate::ganger::rederive_stats_on_injury_change)),
//! plus the hot-reload-survival invariant on the tuning path. Real-path: the
//! projector is the live fn, the systems run in a headless `MinimalPlugins` app.
//!
//! Each test is pin-discriminating against the layer it exercises (a dropped layer or a
//! wrong attribute → derived stat wiring changes the asserted relation).

use bevy::{
    MinimalPlugins,
    prelude::{App, Update, World},
};

use crate::{
    ganger::{
        Aim, Cool, GangerAttributes, Grit, Hp, HpMax, Luck, Reflexes, Shooting, Speed, Strength,
        Toughness, derive_stats, derive_stats_with_injuries, rederive_stats_on_injury_change,
        rederive_stats_on_tuning_change,
    },
    injuries::{
        GainedInjury, InflictedInjuries, InjuryEffect, InjuryName, InspectText, StatDelta,
        StatTarget,
    },
    sample_cone::concentration_p,
    tuning::{ConcentrationCoeffs, GangerStatTuning, StatWeight},
    weapon::Accuracy,
};

/// An arbitrary (NOT shipped) set of distinct attribute magnitudes, so each derived
/// stat's terms are individually distinguishable (mirrors the GTW-384 derive test).
fn sample_attributes() -> GangerAttributes {
    GangerAttributes {
        speed:     Speed::new(4.0),
        aim:       Aim::new(6.0),
        strength:  Strength::new(5.0),
        toughness: Toughness::new(11.0),
        reflexes:  Reflexes::new(3.0),
        cool:      Cool::new(7.0),
        grit:      Grit::new(20.0),
        luck:      Luck::new(2.0),
    }
}

/// Build a [`GainedInjury`] carrying exactly the given effects — the persistent ledger
/// entry shape (its display texts are inert for these projection tests).
fn injury(effects: Vec<InjuryEffect>) -> GainedInjury {
    GainedInjury::new(
        InjuryName::new("test".to_owned()),
        crate::armor::BodyPart::Torso,
        crate::severity::Severity::Major,
        effects,
        InspectText::new("x".to_owned()),
    )
}

/// A ledger pre-loaded with the given effects (folded through the real
/// [`InflictedInjuries::gain`] so the summed-delta store matches a live infliction).
fn ledger_with(effects: Vec<InjuryEffect>) -> InflictedInjuries {
    let mut ledger = InflictedInjuries::default();
    ledger.gain(injury(effects));
    ledger
}

/// Test (identity) — an EMPTY ledger derives EXACTLY as the pure GTW-384
/// [`derive_stats`] (the zero-delta identity), so a ganger with no injuries is
/// unchanged. Pin-discriminating: any spurious delta would diverge the two.
#[test]
fn empty_ledger_is_the_zero_delta_identity() {
    let tuning = GangerStatTuning::default();
    let a = sample_attributes();
    let plain = derive_stats(&a, &tuning);
    let injured = derive_stats_with_injuries(&a, &tuning, &InflictedInjuries::default());
    assert_eq!(
        plain, injured,
        "an empty injury ledger must derive exactly as derive_stats (zero-delta identity)",
    );
}

/// Test (1) — a `Modify(attribute)` ledger entry RIPPLES into every derived stat that
/// reads that attribute. A `Modify(Aim, -3)` lowers the derived Shooting (Aim feeds
/// Shooting), and exactly matches deriving from the hand-lowered base Aim — proving the
/// attribute delta folds in PRE-derivation. Pin-discriminating: a POST-only application
/// (skipping the ripple) would leave Shooting at its base-Aim value.
#[test]
fn attribute_modify_ripples_through_derived_stats() {
    let tuning = GangerStatTuning::default();
    let base = sample_attributes();
    let delta: i8 = -3;

    let injured = derive_stats_with_injuries(
        &base,
        &tuning,
        &ledger_with(vec![InjuryEffect::Modify {
            stat:   StatTarget::Aim,
            amount: StatDelta::new(delta),
        }]),
    );

    // The expected: derive from the base with Aim lowered by the same delta (the ripple).
    let mut lowered = base;
    lowered.aim = Aim::new(*base.aim + f32::from(delta));
    let expected = derive_stats(&lowered, &tuning);

    assert_eq!(
        injured.shooting.to_bits(),
        expected.shooting.to_bits(),
        "a Modify(Aim) must ripple through the derived Shooting (PRE-derivation fold)",
    );
    let plain = derive_stats(&base, &tuning);
    assert!(
        *injured.shooting < *plain.shooting,
        "lowering Aim must lower the derived Shooting (the term wires Aim through)",
    );
}

/// Test (2) — a `Modify(derived)` ADDS on top of that derived stat, INDEPENDENT of the
/// attribute layer. A `Modify(Shooting, +5)` raises the derived Shooting by exactly 5
/// over the no-injury value, while Aim (and thus every OTHER Aim-driven term) is
/// untouched. Pin-discriminating: folding it pre-derivation (treating Shooting as an
/// attribute) would not add a clean +5, and would not leave the formula identical.
#[test]
fn derived_modify_adds_on_top_independent_of_attributes() {
    let tuning = GangerStatTuning::default();
    let base = sample_attributes();
    let plain = derive_stats(&base, &tuning);
    let bump: i8 = 5;

    let injured = derive_stats_with_injuries(
        &base,
        &tuning,
        &ledger_with(vec![InjuryEffect::Modify {
            stat:   StatTarget::Shooting,
            amount: StatDelta::new(bump),
        }]),
    );

    let expected = *plain.shooting + f32::from(bump);
    assert_eq!(
        injured.shooting.to_bits(),
        Shooting::new(expected).to_bits(),
        "a Modify(Shooting) must add its delta on top of the derived Shooting",
    );
    // The attribute layer is untouched: every OTHER derived stat is unchanged.
    assert_eq!(
        injured.fight.to_bits(),
        plain.fight.to_bits(),
        "a derived-Shooting Modify must not touch Fight (it is not the attribute layer)",
    );
}

/// A headless app with both re-derive systems in `Update` — the focused GTW-436 harness
/// (the systems self-guard on the `Option<Res<GangerStatTuning>>`).
fn rederive_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).add_systems(
        Update,
        (
            rederive_stats_on_tuning_change,
            rederive_stats_on_injury_change,
        ),
    );
    app
}

/// Spawn one ganger carrying its eight attribute components + its derived components
/// (seeded by the INJURY-AWARE projector over `ledger`) + the `ledger` itself — exactly
/// the shape the live setup + a gained injury produce. A unit-test world-mutation
/// carve-out (bevy-traps #7a).
fn spawn_injured_ganger(
    world: &mut World,
    a: &GangerAttributes,
    tuning: &GangerStatTuning,
    ledger: InflictedInjuries,
) -> bevy::prelude::Entity {
    let d = derive_stats_with_injuries(a, tuning, &ledger);
    world
        .spawn((
            a.speed,
            a.aim,
            a.strength,
            a.toughness,
            a.reflexes,
            a.cool,
            a.grit,
            a.luck,
        ))
        .insert((d.shooting, d.fight, d.reactions, d.morale))
        .insert((
            d.tu,
            d.tu_max,
            d.hp,
            d.hp_max,
            d.wounds,
            d.wounds_max,
            d.bottle,
        ))
        .insert(ledger)
        .id()
}

/// Test (3) — the HOT-RELOAD-SURVIVAL invariant: a `stat_tuning.ron` re-derive
/// RE-APPLIES (does NOT wipe) the injury deltas. Drive the REAL
/// `rederive_stats_on_tuning_change` with a NON-EMPTY ledger, then assert the
/// re-derived stat equals `f(base, NEW tuning, ledger)` — the deltas survive by
/// construction. Pin-discriminating: a re-derive that ignored the ledger would land on
/// `derive_stats(base, NEW tuning)` (the no-injury value), which differs.
#[test]
fn tuning_hot_reload_reapplies_injury_deltas() {
    let mut app = rederive_app();
    let attrs = sample_attributes();
    let baseline = GangerStatTuning::default();
    // A Modify(Aim) injury — its delta must survive the tuning re-derive.
    let ledger = ledger_with(vec![InjuryEffect::Modify {
        stat:   StatTarget::Aim,
        amount: StatDelta::new(-4),
    }]);
    app.world_mut().insert_resource(baseline.clone());
    let entity = spawn_injured_ganger(app.world_mut(), &attrs, &baseline, ledger.clone());
    app.update();

    // Hot-edit the tuning: a NEW Aim weight (a distinct Shooting). The invariant says the
    // re-derive lands on f(base, NEW tuning, ledger) — the injury delta STILL applied.
    let mut edited = baseline;
    edited.shooting.aim = StatWeight::new((*edited.shooting.aim).mul_add(2.0, 1.0));
    let expected = derive_stats_with_injuries(&attrs, &edited, &ledger).shooting;
    // And the no-injury value the WIPE bug would produce (must differ — proves survival).
    let wiped = derive_stats(&attrs, &edited).shooting;
    app.world_mut().insert_resource(edited);
    app.update();

    let after = app.world().get::<Shooting>(entity).map(|s| **s);
    assert_eq!(
        after,
        Some(*expected),
        "a tuning hot-reload must RE-APPLY the injury delta: stat == f(base, NEW tuning, ledger)",
    );
    assert_ne!(
        after,
        Some(*wiped),
        "the re-derived stat must NOT equal the no-injury value (the delta was not wiped)",
    );
}

/// GTW-443 C8 — the 1H aim penalty composes through the cached `Shooting` with ZERO new
/// plumbing: a `[DisableHand, Modify(Shooting, -N)]` injury drops the live `Shooting`
/// component by EXACTLY `N` after ONE rederive tick, and the resulting §1b concentration
/// exponent is measurably LOOSER (less centered → a SMALLER `p`) than the uninjured
/// baseline at a fixed weapon accuracy.
///
/// MUST settle one schedule tick between gaining the injury and reading (the
/// `Changed<InflictedInjuries>` rederive runs in `Update`), per the settle-before-read
/// rule — reading the same tick would race the projection.
///
/// Pin-discriminating: the `DisableHand` sibling effect is INERT for the stat layer, so the
/// drop is EXACTLY `N` (a leak from `DisableHand` would over/under-shoot); and a smaller
/// `Shooting` yields a strictly smaller `concentration_p` (the looser cone), so an
/// accidental always-on or absent penalty would break the inequality.
#[test]
fn one_handed_aim_penalty_composes_through_shooting_and_loosens_the_cone() {
    let mut app = rederive_app();
    let attrs = sample_attributes();
    let tuning = GangerStatTuning::default();
    app.world_mut().insert_resource(tuning.clone());
    // Spawn uninjured first (the baseline cached Shooting), then inflict the hand injury.
    let entity = spawn_injured_ganger(
        app.world_mut(),
        &attrs,
        &tuning,
        InflictedInjuries::default(),
    );
    app.update();
    let baseline_shooting = app.world().get::<Shooting>(entity).map(|s| **s);
    let Some(baseline_shooting) = baseline_shooting else {
        return;
    };

    // Inflict the hand-disabling injury: BOTH a DisableHand (inert for the stat layer) AND
    // the always-on Modify(Shooting, -N) aim penalty in ONE effects Vec (the GTW-443 F3
    // shape). N is arbitrary here (a tunable per-injury authored magnitude, not pinned).
    let penalty: i8 = -2;
    if let Some(mut led) = app.world_mut().get_mut::<InflictedInjuries>(entity) {
        led.gain(injury(vec![
            InjuryEffect::DisableHand,
            InjuryEffect::Modify {
                stat:   StatTarget::Shooting,
                amount: StatDelta::new(penalty),
            },
        ]));
    }
    // SETTLE ONE TICK so the Changed<InflictedInjuries> rederive projects the delta.
    app.update();

    let injured_shooting = app.world().get::<Shooting>(entity).map(|s| **s);
    assert_eq!(
        injured_shooting,
        Some(baseline_shooting + f32::from(penalty)),
        "the cached Shooting drops by EXACTLY N (the DisableHand sibling is inert for the stat layer)",
    );
    let Some(injured_shooting) = injured_shooting else {
        return;
    };

    // The §1b concentration exponent is LOOSER (smaller p) for the injured Shooting — a
    // less-centered cone — at a fixed weapon accuracy. Coefficients from default tuning
    // (none hardcoded); the RELATION is asserted, never a magnitude.
    let accuracy = Accuracy::new(1.0);
    let coeffs = ConcentrationCoeffs::default();
    let baseline_p = concentration_p(Shooting::new(baseline_shooting), accuracy, coeffs);
    let injured_p = concentration_p(Shooting::new(injured_shooting), accuracy, coeffs);
    assert!(
        *injured_p < *baseline_p,
        "a lower Shooting must loosen the cone (a smaller concentration_p): injured {} < baseline {}",
        *injured_p,
        *baseline_p,
    );
}

/// Test (4) — a POOL `Modify` docks the derived MAX and CLAMPS current via `min`, and
/// can NEVER reduce current to 0 (never self-kills). Drive the REAL
/// `rederive_stats_on_injury_change`: a ganger gains a huge negative `Modify(Hp)`; assert
/// `HpMax` is docked (and floored ≥ 1), current Hp is clamped to the new max via min, and
/// current Hp stays ≥ 1 (alive). Pin-discriminating: an unfloored dock would drive the
/// max (and clamped current) to 0; a reset-not-clamp would restore a damaged pool.
#[test]
fn pool_modify_docks_max_clamps_current_never_self_kills() {
    let mut app = rederive_app();
    let attrs = sample_attributes();
    let tuning = GangerStatTuning::default();
    app.world_mut().insert_resource(tuning.clone());
    // Spawn with NO injuries first (full pools), then inflict the pool Modify live.
    let entity = spawn_injured_ganger(
        app.world_mut(),
        &attrs,
        &tuning,
        InflictedInjuries::default(),
    );
    app.update();
    let base_max = app.world().get::<HpMax>(entity).map(|m| **m);

    // Inflict a MASSIVE Hp dock (far below 0) — the floor + no-self-kill guard must hold.
    if let Some(mut led) = app.world_mut().get_mut::<InflictedInjuries>(entity) {
        led.gain(injury(vec![InjuryEffect::Modify {
            stat:   StatTarget::Hp,
            amount: StatDelta::new(i8::MIN),
        }]));
    }
    app.update();

    let hp_after = app.world().get::<Hp>(entity).map(|h| **h);
    let max_after = app.world().get::<HpMax>(entity).map(|m| **m);
    assert_eq!(
        max_after,
        Some(1),
        "a massive Hp dock floors the derived HpMax at 1 (never 0)",
    );
    assert!(
        base_max.is_some_and(|m| m > 1),
        "precondition: the un-docked HpMax exceeded 1",
    );
    // Current Hp clamps to the new max via min — and stays ≥ 1 (the dock cannot self-kill).
    assert_eq!(
        hp_after,
        Some(1),
        "current Hp clamps to the docked max via min, and a pool Modify can never reach 0",
    );
}
