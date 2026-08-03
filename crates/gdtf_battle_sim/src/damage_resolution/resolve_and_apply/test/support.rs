pub(super) use bevy::prelude::{Entity, World};

pub(super) use crate::{
    apply_hit::{GangerHitTarget, apply_hit},
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType, BodyPart,
    },
    armor_wear::{ArmorBroken, ArmorDamaged, ArmorWearOutcome},
    central_axis::climb_aim_dir,
    cone::{ConeAngle, PriorShots},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::{Hp, LifeState, Luck, Toughness, Wounds},
    inflicted_wound::InflictedWounds,
    injuries::{InjuryRegistry, InjuryTables},
    magazine::{Magazine, ReloadTu},
    matchup::{Matchup, matchup},
    metric::{Cell, CellLevel, Level, SimPos},
    resolve_and_apply::{
        AppliedDamage, CoverVerdict, GangerVerdict, GroundAccrual, HitReport, HitVerdict,
        SlabVerdict, StruckPiece, StruckSurfaces, TargetGanger, resolve_and_apply,
    },
    resolve_coarse::{ShotKind, ShotOutcome},
    resolve_hit::{HitResult, resolve_hit},
    rng::{BattleSeed, InjuryRng, SeverityRng},
    sample_cone::{ConcentrationP, ShotDir, sample_cone_vector},
    severity::{Severity, SeverityInputs, part_severity_mod, roll_severity},
    slab::{SlabEntry, SlabHp, SlabLedger},
    stability::RecoilGrowth,
    surface::GroundDamage,
    tuning::{CombatTuning, RecoilClimb},
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, FireModeSpec,
        Handedness, HandlingProfile, Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots,
        ModeTuPercent, Shove, Stable, WeaponBundle, WeaponDamage, WeaponName, WeaponPunch,
        WeaponShred,
    },
};

pub(super) const SEED: u64 = 0x05EE_D191;

pub(super) fn rng() -> SeverityRng {
    SeverityRng::from_root(BattleSeed::new(SEED))
}

pub(super) fn ledger() -> CoverLedger {
    CoverLedger::new()
}

pub(super) fn injury_rng() -> InjuryRng {
    InjuryRng::from_root(BattleSeed::new(SEED))
}

pub(super) fn injury_tables() -> InjuryTables {
    InjuryTables::default()
}

pub(super) fn injury_registry() -> InjuryRegistry {
    InjuryRegistry::default()
}

pub(super) fn an_entity() -> Entity {
    World::new().spawn_empty().id()
}

pub(super) fn ganger_verdict(report: &HitReport) -> Option<&GangerVerdict> {
    match &report.verdict {
        HitVerdict::Ganger(verdict) => Some(verdict),
        HitVerdict::Cover(_)
        | HitVerdict::Slab(_)
        | HitVerdict::Ground(_)
        | HitVerdict::NoEffect => None,
    }
}

pub(super) fn applied_of(report: &HitReport) -> Option<AppliedDamage> {
    ganger_verdict(report).map(|verdict| verdict.applied)
}

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
            Shove::new(false),
            Handedness::OneHanded,
        ),
    )
}

pub(super) fn piece_integrity(integrity: i32) -> ArmorIntegrity {
    ArmorIntegrity::new(integrity)
}

pub(super) fn struck_piece(
    floor: i32,
    protection: i32,
    hardness: i32,
    armor_type: ArmorType,
    integrity: &mut ArmorIntegrity,
) -> StruckPiece<'_> {
    StruckPiece {
        floor: ArmorFloor::new(floor),
        protection: ArmorProtection::new(protection),
        hardness: ArmorHardness::new(hardness),
        armor_type,
        integrity,
    }
}

pub(super) fn a_trajectory() -> ShotDir {
    let aim = climb_aim_dir(
        SimPos::new(0.0, 0.0, 0.5),
        SimPos::new(5.0, 0.0, 0.5),
        PriorShots::new(0),
        RecoilClimb::new(0.0),
        RecoilGrowth::new(0.0),
    );
    sample_cone_vector(
        aim,
        ConeAngle::new(0.0),
        ConcentrationP::new(1.0),
        rng().rng(),
    )
}

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

pub(super) fn cover_cell_level() -> CellLevel {
    CellLevel::new(Cell::new(4, 6), Level::new(2))
}

pub(super) fn cover_entry(max_hp: u32, protection: i32, hardness: i32) -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(max_hp),
        HeightBand::Mid,
        ArmorProtection::new(protection),
        ArmorHardness::new(hardness),
    )
}

pub(super) fn cover_outcome(entry: CoverEntry) -> ShotOutcome {
    let at = cover_cell_level();
    let (cell, level) = at.split();
    ShotOutcome {
        kind: ShotKind::Cover(entry),
        cell,
        level,
        body_part: None,
        band: HeightBand::Mid,
        muzzle: SimPos::new(0.5, 0.5, 0.5),
        trajectory: a_trajectory(),
    }
}

pub(super) fn slab_ledger() -> SlabLedger {
    SlabLedger::new()
}

pub(super) fn surfaces<'a>(
    cover: &'a mut CoverLedger,
    slab: &'a mut SlabLedger,
) -> StruckSurfaces<'a> {
    StruckSurfaces { cover, slab }
}

pub(super) fn slab_cell_level() -> CellLevel {
    CellLevel::new(Cell::new(5, 7), Level::new(1))
}

pub(super) fn slab_entry(max_hp: u32, protection: i32, hardness: i32) -> SlabEntry {
    SlabEntry::seeded(
        SlabHp::new(max_hp),
        ArmorProtection::new(protection),
        ArmorHardness::new(hardness),
    )
}

pub(super) fn slab_outcome() -> ShotOutcome {
    let at = slab_cell_level();
    let (cell, level) = at.split();
    ShotOutcome {
        kind: ShotKind::Slab(at),
        cell,
        level,
        body_part: None,
        band: HeightBand::Low,
        muzzle: SimPos::new(0.5, 0.5, 0.5),
        trajectory: a_trajectory(),
    }
}

pub(super) fn ground_cell_level() -> CellLevel {
    CellLevel::new(Cell::new(2, 9), Level::new(0))
}

pub(super) fn ground_outcome() -> ShotOutcome {
    let at = ground_cell_level();
    let (cell, level) = at.split();
    ShotOutcome {
        kind: ShotKind::Ground(at),
        cell,
        level,
        body_part: None,
        band: HeightBand::Low,
        muzzle: SimPos::new(0.5, 0.5, 0.5),
        trajectory: a_trajectory(),
    }
}
