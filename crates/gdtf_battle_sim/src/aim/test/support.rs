//! Shared test fixtures for the composer tests — the [`ShooterState`] borrow
//! source, arbitrary cover / weapon builders, and the faced-cover ledger helper.

use crate::{
    aim::Shooter,
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    faced_cell::faced_cell,
    ganger::{Aiming, Direction, Facing, Position, Stance, StanceKind},
    metric::{Cell, CellLevel, Level},
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, FireModeSpec,
        HandlingProfile, Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots, ModeTuPercent,
        Stable, WeaponBundle, WeaponDamage, WeaponName, WeaponPunch, WeaponShred,
    },
};

/// Build a shooter read-state from owned components the test holds — the
/// borrows the [`Shooter`] bundle wants are taken from these locals.
pub(super) struct ShooterState {
    pub(super) stance:   Stance,
    pub(super) aiming:   Aiming,
    pub(super) position: Position,
    pub(super) facing:   Facing,
}

impl ShooterState {
    /// An arbitrary shooter at `(x, y, storey)` with the given posture / aim /
    /// facing — NOT shipped magnitudes; only its components matter.
    pub(super) fn new(
        x: i32,
        y: i32,
        storey: u8,
        kind: StanceKind,
        aiming: bool,
        dir: Direction,
    ) -> Self {
        Self {
            stance:   Stance::new(kind),
            aiming:   Aiming::new(aiming),
            position: Position::new(CellLevel::new(Cell::new(x, y), Level::new(storey))),
            facing:   Facing::new(dir),
        }
    }

    /// Borrow the owned components as a [`Shooter`] bundle.
    pub(super) fn as_shooter(&self) -> Shooter<'_> {
        Shooter {
            stance:   &self.stance,
            aiming:   &self.aiming,
            position: &self.position,
            facing:   &self.facing,
        }
    }
}

/// An arbitrary faced-cover entry at `band` — NOT shipped magnitudes; the
/// stability layer only reads `height_band`, so the HP / armor are filler.
pub(super) fn cover_entry(band: HeightBand) -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(10),
        band,
        ArmorProtection::new(1),
        ArmorHardness::new(1),
    )
}

/// An arbitrary single-mode armed-entity bundle with the given `stable` tag —
/// NOT shipped magnitudes (there are no shipped weapons yet); only its
/// `base_spread` / `kickback` / `fire_mode` / `stable` flow through the
/// composers. Returns the owned [`WeaponBundle`]; the call site assembles the
/// [`crate::weapon::WeaponStats`] read-view via [`WeaponBundle::stats`].
pub(super) fn weapon_tagged(base: f32, kick: f32, stable: bool) -> WeaponBundle {
    WeaponBundle::new(
        WeaponName::new("test-weapon".to_owned()),
        BaseSpread::new(base),
        Accuracy::new(1.0),
        Kickback::new(kick),
        FatalBias::new(0.0),
        DamageProfile::new(
            WeaponDamage::new(10),
            WeaponPunch::new(2),
            WeaponShred::new(1),
            DamageType::Kinetic,
        ),
        HandlingProfile::new(
            MagazineSize::new(10),
            FireMode::new(vec![FireModeSpec::new(
                ModeKind::Single,
                ModeConeMult::new(1.0),
                ModeTuPercent::new(0.5),
                ModeShots::new(1),
            )]),
            Stable::new(stable),
        ),
    )
}

/// An arbitrary NON-stable single-mode bundle — the default for tests that do
/// not exercise the `stable` tag.
pub(super) fn weapon(base: f32, kick: f32) -> WeaponBundle {
    weapon_tagged(base, kick, false)
}

/// Insert `entry` into a fresh ledger at the cell `shooter` faces, so the
/// composer's `peek` finds it as the faced cover.
pub(super) fn ledger_with_faced_cover(shooter: &Shooter, entry: CoverEntry) -> CoverLedger {
    let (cell, level) = faced_cell(shooter.position, shooter.facing);
    let mut ledger = CoverLedger::new();
    ledger.insert(CellLevel::new(cell, level), entry);
    ledger
}
