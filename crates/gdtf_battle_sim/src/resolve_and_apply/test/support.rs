//! Shared fixtures for the E3.9 fold tests — the seeded RNG, a throwaway entity,
//! an armed weapon bundle, a uniform worn suit, a real-pipeline trajectory, and the
//! ganger / non-ganger [`ShotOutcome`] builders — plus the `pub(super)` re-export
//! of the symbols the AC test files exercise. Relocated verbatim from the flat
//! module's `tests` submodule.

pub(super) use bevy::prelude::{Entity, World};

pub(super) use crate::{
    apply_hit::{GangerHitTarget, apply_hit},
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType, BodyPart,
    },
    armor_wear::{ArmorBroken, ArmorWearOutcome, ArmorWorn},
    central_axis::climb_aim_dir,
    cone::{ConeAngle, PriorShots},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::{Hp, LifeState, Luck, Toughness, Wounds},
    inflicted_wound::InflictedWounds,
    magazine::{Magazine, ReloadTu},
    matchup::{Matchup, matchup},
    metric::{Cell, CellLevel, Level, SimPos},
    resolve_and_apply::{
        AppliedDamage, HitReport, StruckPiece, StruckSurfaces, TargetGanger, resolve_and_apply,
    },
    resolve_coarse::{ShotKind, ShotOutcome},
    resolve_hit::{HitResult, resolve_hit},
    rng::{BattleSeed, SimRng},
    sample_cone::{ConcentrationP, ShotDir, sample_cone_vector},
    severity::{Severity, SeverityInputs, part_severity_mod, roll_severity},
    slab::{SlabEntry, SlabHp, SlabLedger},
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

/// A fresh, empty [`CoverLedger`] for the ganger-path fold tests — those outcomes never
/// strike cover, so the ledger is threaded only to satisfy the `resolve_and_apply`
/// signature (GTW-364) and is never read or written by a ganger / corpse / non-cover fold.
pub(super) fn ledger() -> CoverLedger {
    CoverLedger::new()
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

/// The starting [`ArmorIntegrity`] for a struck-piece fixture — an arbitrary (NOT
/// shipped-tuning) magnitude. Since GTW-323 (ADR-0004) the fold reads + wears the
/// struck piece's integrity component by `&mut`, so the tests hold this local and lend
/// it to a [`StruckPiece`] via [`struck_piece`].
pub(super) fn piece_integrity(integrity: i32) -> ArmorIntegrity {
    ArmorIntegrity::new(integrity)
}

/// Build a [`StruckPiece`] borrow-view from arbitrary (NOT shipped-tuning) stats and
/// the caller's mutable [`ArmorIntegrity`] local — the piece-entity replacement for
/// the old uniform `worn_suit`, so a hit lands in a known regime.
///
/// The caller owns the integrity (a `let mut` local) and lends it here so the view's
/// `&mut` wear borrow outlives the `resolve_and_apply` call; the four read stats are
/// passed by value.
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

/// The `(cell, level)` the cover-hit fixtures place their struck cover at — a fixed
/// arbitrary key, shared by the fixture's seeded ledger entry and its outcome so
/// `deplete_cover` keys the same cell (zero px).
pub(super) fn cover_cell_level() -> CellLevel {
    CellLevel::new(Cell::new(4, 6), Level::new(2))
}

/// A seeded full-HP [`CoverEntry`] at `max_hp` HP with the given armor stats (band
/// MID — irrelevant to the damage formula). The fixtures pin armor magnitudes
/// (protection / hardness) deliberately LOW so the chosen weapon damage controls
/// whether the hit destroys — the MECHANISM under test, not a balanced magnitude.
pub(super) fn cover_entry(max_hp: u32, protection: i32, hardness: i32) -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(max_hp),
        HeightBand::Mid,
        ArmorProtection::new(protection),
        ArmorHardness::new(hardness),
    )
}

/// A [`ShotKind::Cover`] outcome carrying `entry`, struck at [`cover_cell_level`]
/// (the cell/level the cover-hit path keys `deplete_cover` with). No body part (the
/// §4 roll never runs on cover); the muzzle / trajectory are arbitrary (zero px).
pub(super) fn cover_outcome(entry: CoverEntry) -> ShotOutcome {
    let at = cover_cell_level();
    ShotOutcome {
        kind:       ShotKind::Cover(entry),
        cell:       Cell::new(at.x, at.y),
        level:      Level::new(u8::try_from(at.z).unwrap_or(0)),
        body_part:  None,
        band:       HeightBand::Mid,
        muzzle:     SimPos::new(0.5, 0.5, 0.5),
        trajectory: a_trajectory(),
    }
}

/// A fresh, empty [`SlabLedger`] for the fold tests (GTW-365) — a struck slab lazily
/// seeds from the `SlabDefaults` tuning leaf, so an empty ledger is the right start.
/// Threaded only to satisfy the [`StruckSurfaces`] bundle on the ganger / cover paths
/// (those never touch the slab ledger).
pub(super) fn slab_ledger() -> SlabLedger {
    SlabLedger::new()
}

/// Bundle two ledgers into the [`StruckSurfaces`] argument
/// [`resolve_and_apply`] now takes (GTW-365) — the shared shape every fold-test call
/// site builds. The fold's `ShotKind` selects which ledger a hit touches.
pub(super) fn surfaces<'a>(
    cover: &'a mut CoverLedger,
    slab: &'a mut SlabLedger,
) -> StruckSurfaces<'a> {
    StruckSurfaces { cover, slab }
}

/// The `(cell, level)` the slab-hit fixtures place their struck slab at — a fixed
/// arbitrary key, the key `deplete_slab` records on destruction.
pub(super) fn slab_cell_level() -> CellLevel {
    CellLevel::new(Cell::new(5, 7), Level::new(1))
}

/// A seeded full-HP [`SlabEntry`] at `max_hp` HP with the given armor stats — the
/// prototype a fixture inserts so a low-HP slab can be breached in a single hit (the
/// fixtures pin armor LOW so the chosen weapon damage controls destruction — the
/// MECHANISM under test, not a balanced magnitude).
pub(super) fn slab_entry(max_hp: u32, protection: i32, hardness: i32) -> SlabEntry {
    SlabEntry::seeded(
        SlabHp::new(max_hp),
        ArmorProtection::new(protection),
        ArmorHardness::new(hardness),
    )
}

/// A [`ShotKind::Slab`] outcome struck at [`slab_cell_level`] (the surface cell the
/// slab-hit path keys `deplete_slab` with). `ShotKind::Slab` carries the slab key
/// directly (not the entry — slabs lazily seed from tuning); no body part.
pub(super) fn slab_outcome() -> ShotOutcome {
    let at = slab_cell_level();
    ShotOutcome {
        kind:       ShotKind::Slab(at),
        cell:       Cell::new(at.x, at.y),
        level:      Level::new(u8::try_from(at.z).unwrap_or(0)),
        body_part:  None,
        band:       HeightBand::Low,
        muzzle:     SimPos::new(0.5, 0.5, 0.5),
        trajectory: a_trajectory(),
    }
}
