use crate::{
    aim::Shooter,
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    faced_cell::faced_cell,
    ganger::{Aiming, Direction, Facing, Position, Stance, StanceKind, Suppressed, SuppressorCell},
    magazine::{Magazine, ReloadTu},
    metric::{Cell, CellLevel, Level},
    stability::StabilityTerms,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, FireModeSpec,
        Handedness, HandlingProfile, Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots,
        ModeTuPercent, Shove, Stable, WeaponBundle, WeaponDamage, WeaponName, WeaponPunch,
        WeaponShred,
    },
};

pub(super) struct ShooterState {
    pub(super) stance:     Stance,
    pub(super) aiming:     Aiming,
    pub(super) position:   Position,
    pub(super) facing:     Facing,
    pub(super) suppressed: Option<Suppressed>,
}

impl ShooterState {
    pub(super) fn new(
        x: i32,
        y: i32,
        storey: u8,
        kind: StanceKind,
        aiming: bool,
        dir: Direction,
    ) -> Self {
        Self {
            stance:     Stance::new(kind),
            aiming:     Aiming::new(aiming),
            position:   Position::new(CellLevel::new(Cell::new(x, y), Level::new(storey))),
            facing:     Facing::new(dir),
            suppressed: None,
        }
    }

    pub(super) fn suppressed_from(mut self, sx: i32, sy: i32, storey: u8) -> Self {
        let origin = CellLevel::new(Cell::new(sx, sy), Level::new(storey));
        self.suppressed = Some(Suppressed::new(SuppressorCell::new(origin)));
        self
    }

    pub(super) fn as_shooter(&self) -> Shooter<'_> {
        Shooter {
            stance:     &self.stance,
            aiming:     &self.aiming,
            position:   &self.position,
            facing:     &self.facing,
            suppressed: self.suppressed.as_ref(),
        }
    }
}

pub(super) fn cover_entry(band: HeightBand) -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(10),
        band,
        ArmorProtection::new(1),
        ArmorHardness::new(1),
    )
}

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
            Magazine::loaded(MagazineSize::new(10), ReloadTu::new(10)),
            FireMode::new(vec![FireModeSpec::new(
                ModeKind::Single,
                ModeConeMult::new(1.0),
                ModeTuPercent::new(0.5),
                ModeShots::new(1),
            )]),
            Stable::new(stable),
            Shove::new(false),
            Handedness::OneHanded,
        ),
    )
}

pub(super) fn weapon(base: f32, kick: f32) -> WeaponBundle {
    weapon_tagged(base, kick, false)
}

pub(super) fn weapon_terms(wpn: &WeaponBundle) -> StabilityTerms {
    StabilityTerms {
        stable: wpn.stable,
        ..StabilityTerms::default()
    }
}

pub(super) fn ledger_with_faced_cover(shooter: &Shooter, entry: CoverEntry) -> CoverLedger {
    let (cell, level) = faced_cell(shooter.position, shooter.facing);
    let mut ledger = CoverLedger::new();
    ledger.insert(CellLevel::new(cell, level), entry);
    ledger
}
