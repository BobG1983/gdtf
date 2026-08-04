use bevy::prelude::World;

use super::{InflictedWound, InflictedWounds};
use crate::{
    armor::BodyPart,
    central_axis::climb_aim_dir,
    cone::{ConeAngle, PriorShots},
    cover::{CoverLedger, HeightBand},
    ganger::{Hp, LifeState, Luck, Toughness, Wounds},
    injuries::{InjuryRegistry, InjuryTables},
    magazine::{Magazine, ReloadTu},
    metric::{Cell, Level, SimPos},
    resolve_and_apply::{
        HitVerdict, ShotSource, StruckSurfaces, TargetGanger, WoundRoll, resolve_and_apply,
    },
    resolve_coarse::{ShotKind, ShotOutcome},
    rng::{BattleSeed, InjuryRng, SeverityRng},
    sample_cone::{ConcentrationP, ShotDir, sample_cone_vector},
    severity::Severity,
    slab::SlabLedger,
    stability::RecoilGrowth,
    tuning::{CombatTuning, RecoilClimb},
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, FireModeSpec,
        Handedness, HandlingProfile, Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots,
        ModeTuPercent, Shove, Stable, WeaponBundle, WeaponDamage, WeaponName, WeaponPunch,
        WeaponShred,
    },
};

const SEED: u64 = 0x1FF1_1C7E;

fn rng() -> SeverityRng {
    SeverityRng::from_root(BattleSeed::new(SEED))
}

fn an_entity() -> bevy::prelude::Entity {
    World::new().spawn_empty().id()
}

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
            Magazine::loaded(MagazineSize::new(10), ReloadTu::new(10)),
            FireMode::new(vec![FireModeSpec::new(
                ModeKind::Single,
                ModeConeMult::new(1.0),
                ModeTuPercent::new(1.0),
                ModeShots::new(1),
            )]),
            Stable::new(false),
            Shove::new(false),
            Handedness::OneHanded,
        ),
    )
}

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

#[test]
fn fresh_inflicted_wounds_is_empty() {
    let fresh = InflictedWounds::default();
    assert!(
        fresh.is_empty(),
        "a freshly spawned ganger must carry an empty InflictedWounds ",
    );
    assert_eq!(fresh.len(), 0, "an empty record has length 0");
}

#[test]
fn damaging_hits_accumulate_in_order_with_the_rolled_tier_and_part() {
    let tuning = CombatTuning::default();
    let entity = an_entity();
    let weapon = a_weapon();
    let parts = [
        BodyPart::Torso,
        BodyPart::LeftArm,
        BodyPart::Head,
        BodyPart::RightLeg,
    ];

    let mut hp = Hp::new(255);
    let mut wounds = Wounds::new(255);
    let mut life = LifeState::Alive;
    let mut inflicted = InflictedWounds::default();
    let mut r = rng();
    let mut cover = CoverLedger::new();
    let mut slab = SlabLedger::new();

    let mut expected: Vec<InflictedWound> = Vec::new();

    for &part in &parts {
        let before = inflicted.len();

        let report = resolve_and_apply(
            &ganger_outcome(entity, part),
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
            StruckSurfaces {
                cover: &mut cover,
                slab:  &mut slab,
            },
            &mut WoundRoll {
                tuning:       &tuning,
                severity_rng: &mut r,
                tables:       &InjuryTables::default(),
                registry:     &InjuryRegistry::default(),
                injury_rng:   &mut InjuryRng::from_root(BattleSeed::new(SEED)),
            },
        );

        let HitVerdict::Ganger(verdict) = &report.verdict else {
            continue;
        };
        if verdict.applied.severity == Severity::None {
            assert_eq!(
                inflicted.len(),
                before,
                "a graze (Severity::None) must NOT grow the InflictedWounds list",
            );
        } else {
            assert_eq!(
                inflicted.len(),
                before + 1,
                "a non-graze hit must grow the InflictedWounds list by exactly one",
            );
            expected.push(InflictedWound::new(verdict.applied.severity, verdict.part));
        }
    }

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
