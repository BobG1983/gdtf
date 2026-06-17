//! Tests for the GTW-279 inflicted-wound record — the empty-by-default seed and the
//! accumulation of recorded wounds through the REAL resolution path
//! ([`resolve_and_apply`]), asserted against the resolution's OWN rolled tier +
//! struck part (a seeded RNG, never a hand-set fixture).

use bevy::prelude::World;

use super::{InflictedWound, InflictedWounds};
use crate::{
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType,
        BodyPart, SourceArmor, WornArmor,
    },
    central_axis::climb_aim_dir,
    cone::{ConeAngle, PriorShots},
    cover::HeightBand,
    ganger::{Hp, LifeState, Luck, Toughness, Wounds},
    metric::{Cell, Level, SimPos},
    resolve_and_apply::{TargetGanger, resolve_and_apply},
    resolve_coarse::{ShotKind, ShotOutcome},
    rng::{BattleSeed, SimRng},
    sample_cone::{ConcentrationP, ShotDir, sample_cone_vector},
    severity::Severity,
    stability::RecoilGrowth,
    tuning::{CombatTuning, RecoilClimb},
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, FireModeSpec,
        HandlingProfile, Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
        Stable, WeaponBundle, WeaponDamage, WeaponName, WeaponPunch, WeaponShred,
    },
};

/// A fixed seed so the rolled tier/location are deterministic to assert against.
const SEED: u64 = 0x1FF1_1C7E;

/// A fresh `SimRng` from the shared seed.
fn rng() -> SimRng {
    SimRng::from_seed(BattleSeed::new(SEED))
}

/// A valid `Entity` id from a throwaway world (no hand-crafted raw id, no `unwrap`).
fn an_entity() -> bevy::prelude::Entity {
    World::new().spawn_empty().id()
}

/// A bare-flesh-bypassing weapon with enough damage to actually wound — arbitrary
/// (non-shipped) magnitudes; the §1 cone numbers are irrelevant to E3.9.
fn a_weapon() -> WeaponBundle {
    WeaponBundle::new(
        WeaponName::new("test-weapon".to_owned()),
        BaseSpread::new(0.1),
        Accuracy::new(1.0),
        Kickback::new(0.0),
        FatalBias::new(0.0),
        DamageProfile::new(
            WeaponDamage::new(40),
            WeaponPunch::new(30),
            WeaponShred::new(10),
            DamageType::Kinetic,
        ),
        HandlingProfile::new(
            MagazineSize::new(10),
            FireMode::new(vec![FireModeSpec::new(
                ModeKind::Single,
                ModeConeMult::new(1.0),
                ModeTuPercent::new(1.0),
                ModeShots::new(1),
            )]),
            Stable::new(false),
        ),
    )
}

/// A worn suit worn-through at every piece (integrity 0 ⇒ bare flesh) so a hit lands
/// full damage and reliably wounds.
fn bare_suit() -> WornArmor {
    WornArmor::seed_from(&SourceArmor::uniform(ArmorPiece::new(
        ArmorFloor::new(0),
        ArmorProtection::new(0),
        ArmorIntegrity::new(0),
        ArmorHardness::new(0),
        ArmorType::DEFAULT,
    )))
}

/// A `ShotKind::Ganger` outcome on `entity` struck at `part` — minted through the
/// real cone pipeline for its trajectory (E3.9 reads only `kind` + `body_part`).
fn ganger_outcome(entity: bevy::prelude::Entity, part: BodyPart) -> ShotOutcome {
    let aim = climb_aim_dir(
        SimPos::new(0.0, 0.0, 0.5),
        SimPos::new(5.0, 0.0, 0.5),
        PriorShots::new(0),
        RecoilClimb::new(0.0),
        RecoilGrowth::new(0.0),
    );
    let trajectory: ShotDir = sample_cone_vector(
        aim,
        ConeAngle::new(0.0),
        ConcentrationP::new(1.0),
        rng().rng(),
    );
    ShotOutcome {
        kind: ShotKind::Ganger(entity),
        cell: Cell::new(3, 4),
        level: Level::new(0),
        body_part: Some(part),
        band: HeightBand::Mid,
        muzzle: SimPos::new(0.5, 0.5, 0.5),
        trajectory,
    }
}

/// AC2 — a freshly spawned `InflictedWounds` (the [`Default`] seeded onto every
/// ganger in `setup_battle`) is empty: no recorded wound until one is inflicted.
#[test]
fn fresh_inflicted_wounds_is_empty() {
    let fresh = InflictedWounds::default();
    assert!(
        fresh.is_empty(),
        "a freshly spawned ganger must carry an empty InflictedWounds (GTW-279 AC2)",
    );
    assert_eq!(fresh.len(), 0, "an empty record has length 0");
}

/// AC3 — through the REAL resolution path ([`resolve_and_apply`]), a sequence of
/// damaging hits on one target ACCUMULATES into its `InflictedWounds` in order, and
/// each recorded entry carries the tier the resolution ACTUALLY rolled + the part it
/// struck (asserted against the report's OWN values, not a hand-set fixture). A
/// non-graze hit appends exactly one entry; a graze appends none.
#[test]
fn damaging_hits_accumulate_in_order_with_the_rolled_tier_and_part() {
    let tuning = CombatTuning::default();
    let entity = an_entity();
    let weapon = a_weapon();
    // A sequence of struck parts; the seeded RNG fixes the tier rolled for each.
    let parts = [
        BodyPart::Torso,
        BodyPart::LeftArm,
        BodyPart::Head,
        BodyPart::RightLeg,
    ];

    // A tough enough target (large pools, low Toughness) that the burst keeps
    // wounding without going Dead mid-sequence (a corpse-skip would halt recording).
    let mut hp = Hp::new(255);
    let mut wounds = Wounds::new(255);
    let mut life = LifeState::Alive;
    let mut worn = bare_suit();
    let mut inflicted = InflictedWounds::default();
    let mut r = rng();

    // The tiers the resolution rolls — collected from each report so the assertion
    // pins the record against the resolution's OWN output, never a fixture.
    let mut expected: Vec<InflictedWound> = Vec::new();

    for &part in &parts {
        // Snapshot the record length before this hit so we can assert it grew (or
        // not, on a graze) by exactly the right amount.
        let before = inflicted.len();

        let report = resolve_and_apply(
            &ganger_outcome(entity, part),
            weapon.stats(),
            Luck::new(0.0),
            TargetGanger {
                hp:        &mut hp,
                wounds:    &mut wounds,
                life:      &mut life,
                worn:      &mut worn,
                inflicted: &mut inflicted,
                toughness: Toughness::new(0.0),
                luck:      Luck::new(0.0),
            },
            entity,
            &tuning,
            &mut r,
        );

        let Some(applied) = report.applied else {
            // A ganger hit on a live target always carries an applied block; if it
            // somehow did not, there is nothing to assert for this round.
            continue;
        };
        if applied.severity == Severity::None {
            // A graze records nothing — the list must not have grown this round.
            assert_eq!(
                inflicted.len(),
                before,
                "a graze (Severity::None) must NOT grow the InflictedWounds list",
            );
        } else {
            // A real wound appends exactly one entry carrying the report's OWN
            // rolled tier + the part it names.
            assert_eq!(
                inflicted.len(),
                before + 1,
                "a non-graze hit must grow the InflictedWounds list by exactly one",
            );
            let Some(struck) = report.part else {
                continue;
            };
            expected.push(InflictedWound::new(applied.severity, struck));
        }
    }

    // The full recorded list equals the resolution's own (tier, part) sequence, in
    // infliction order — accumulation + order preserved (GTW-279 AC3).
    assert_eq!(
        inflicted.as_slice(),
        expected.as_slice(),
        "InflictedWounds must accumulate the resolution's own (tier, part) per hit, in order",
    );
    assert!(
        !expected.is_empty(),
        "fixture sanity: the seeded sequence must inflict at least one real wound",
    );
}
