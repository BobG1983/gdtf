//! Shared fixtures for the E3.9 fold tests — the seeded RNG, a throwaway entity,
//! an armed weapon bundle, a uniform worn suit, a real-pipeline trajectory, and the
//! ganger / non-ganger [`ShotOutcome`] builders — plus the `pub(super)` re-export
//! of the symbols the AC test files exercise. Relocated verbatim from the flat
//! module's `tests` submodule.

pub(super) use bevy::prelude::{Entity, World};

pub(super) use crate::{
    apply_hit::{GangerHitTarget, apply_hit},
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorSpec,
        ArmorType, BodyPart, WornArmor,
    },
    armor_wear::{ArmorBroken, ArmorWearOutcome, ArmorWorn},
    central_axis::climb_aim_dir,
    cone::{ConeAngle, PriorShots},
    cover::{CoverEntry, CoverHp, HeightBand},
    ganger::{Hp, LifeState, Luck, Toughness, Wounds},
    inflicted_wound::InflictedWounds,
    magazine::{Magazine, ReloadTu},
    matchup::{Matchup, matchup},
    metric::{Cell, CellLevel, Level, SimPos},
    resolve_and_apply::{AppliedDamage, HitReport, TargetGanger, resolve_and_apply},
    resolve_coarse::{ShotKind, ShotOutcome},
    resolve_hit::{HitResult, resolve_hit},
    rng::{BattleSeed, SimRng},
    sample_cone::{ConcentrationP, ShotDir, sample_cone_vector},
    severity::{Severity, SeverityInputs, part_severity_mod, roll_severity},
    stability::RecoilGrowth,
    tuning::{CombatTuning, RecoilClimb},
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, FireModeSpec,
        HandlingProfile, Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
        Stable, WeaponBundle, WeaponDamage, WeaponName, WeaponPunch, WeaponShred,
    },
};

/// A fixed seed for the per-test RNG streams — determinism is a property, so the
/// same seed must reproduce the same draws (an arbitrary value, not tuned).
pub(super) const SEED: u64 = 0x05EE_D191;

/// Build a `SimRng` from the shared fixed seed (a fresh stream per call).
pub(super) fn rng() -> SimRng {
    SimRng::from_seed(BattleSeed::new(SEED))
}

/// A real, valid [`Entity`] id to stand in for a ganger — spawned from a
/// throwaway [`World`] so the tests never hand-craft a raw id (no `unwrap`).
pub(super) fn an_entity() -> Entity {
    World::new().spawn_empty().id()
}

/// An armed-entity bundle built from arbitrary (NOT shipped-tuning) magnitudes
/// — only the per-hit damage stats and the damage type matter to these tests;
/// the §1 cone/recoil numbers and the fire mode are present but irrelevant here.
/// Returns the owned [`WeaponBundle`]; the call site assembles the
/// [`WeaponStats`](crate::weapon::WeaponStats) read-view via [`WeaponBundle::stats`].
pub(super) fn a_weapon(
    damage: i32,
    punch: i32,
    shred: i32,
    damage_type: DamageType,
) -> WeaponBundle {
    let spec = FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(1.0),
        ModeShots::new(1),
    );
    WeaponBundle::new(
        WeaponName::new("test-weapon".to_owned()),
        BaseSpread::new(0.1),
        Accuracy::new(1.0),
        Kickback::new(0.0),
        FatalBias::new(0.0),
        DamageProfile::new(
            WeaponDamage::new(damage),
            WeaponPunch::new(punch),
            WeaponShred::new(shred),
            damage_type,
        ),
        HandlingProfile::new(
            Magazine::loaded(MagazineSize::new(10), ReloadTu::new(10)),
            FireMode::new(vec![spec]),
            Stable::new(false),
        ),
    )
}

/// A uniform worn suit whose every piece starts at the given stats — arbitrary
/// (NOT shipped-tuning) magnitudes, so a hit lands in a known regime.
pub(super) fn worn_suit(
    floor: i32,
    protection: i32,
    integrity: i32,
    hardness: i32,
    armor_type: ArmorType,
) -> WornArmor {
    WornArmor::seed_from(&ArmorSpec::uniform(ArmorPiece::new(
        ArmorFloor::new(floor),
        ArmorProtection::new(protection),
        ArmorIntegrity::new(integrity),
        ArmorHardness::new(hardness),
        armor_type,
    )))
}

/// A unit-direction [`ShotDir`] fixture — minted through the REAL pipeline
/// ([`climb_aim_dir`] → [`sample_cone_vector`] with a zero cone, which returns
/// the axis exactly and draws nothing), since [`ShotDir`] has no test
/// constructor. The direction is arbitrary (E3.9 never reads `trajectory`); a
/// sim-unit axis, **zero pixels**.
pub(super) fn a_trajectory() -> ShotDir {
    let aim = climb_aim_dir(
        SimPos::new(0.0, 0.0, 0.5),
        SimPos::new(5.0, 0.0, 0.5),
        PriorShots::new(0),
        RecoilClimb::new(0.0),
        RecoilGrowth::new(0.0),
    );
    // A zero cone short-circuits to the axis exactly and consumes no draw.
    sample_cone_vector(
        aim,
        ConeAngle::new(0.0),
        ConcentrationP::new(1.0),
        rng().rng(),
    )
}

/// A `ShotKind::Ganger` outcome on `entity`, struck at `part`. The cell / band /
/// muzzle / trajectory fields are present-but-irrelevant to E3.9 (it reads only
/// `kind` + `body_part`), so they carry arbitrary sim-unit fixtures (zero px).
pub(super) fn ganger_outcome(entity: Entity, part: BodyPart) -> ShotOutcome {
    ShotOutcome {
        kind:       ShotKind::Ganger(entity),
        cell:       Cell::new(3, 4),
        level:      Level::new(0),
        body_part:  Some(part),
        band:       HeightBand::Mid,
        muzzle:     SimPos::new(0.5, 0.5, 0.5),
        trajectory: a_trajectory(),
    }
}

/// A non-ganger outcome of the given `kind` — no body part (the §4 roll runs
/// only on a ganger). The cell / band / muzzle / trajectory are arbitrary.
pub(super) fn non_ganger_outcome(kind: ShotKind) -> ShotOutcome {
    ShotOutcome {
        kind,
        cell: Cell::new(1, 1),
        level: Level::new(0),
        body_part: None,
        band: HeightBand::Low,
        muzzle: SimPos::new(0.5, 0.5, 0.5),
        trajectory: a_trajectory(),
    }
}
