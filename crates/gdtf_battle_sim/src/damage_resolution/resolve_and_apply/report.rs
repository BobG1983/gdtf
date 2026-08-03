//! Report types and mutable views used by resolve_and_apply.

use bevy::prelude::{Deref, Entity};

use crate::{
    armor::{ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType},
    cover::CoverLedger,
    ganger::{Hp, LifeState, Luck, Toughness, Wounds},
    inflicted_wound::InflictedWounds,
    resolve_and_apply::kinds::{
        cover::CoverVerdict, ganger::GangerVerdict, ground::GroundAccrual, slab::SlabVerdict,
    },
    resolve_coarse::ShotKind,
    slab::SlabLedger,
};

/// Mutable view of a living combatant that a hit can change.
pub struct TargetGanger<'a> {
    /// Hit points.
    pub hp: &'a mut Hp,
    /// Wound capacity.
    pub wounds: &'a mut Wounds,
    /// Life state.
    pub life: &'a mut LifeState,
    /// Armor piece at the hit location, if any.
    pub piece: Option<StruckPiece<'a>>,
    /// Wound history.
    pub inflicted: &'a mut InflictedWounds,
    /// Toughness used by severity.
    pub toughness: Toughness,
    /// Luck used by severity.
    pub luck: Luck,
}

/// Mutable armor piece that was struck.
pub struct StruckPiece<'a> {
    /// Armor floor.
    pub floor: ArmorFloor,
    /// Protection value.
    pub protection: ArmorProtection,
    /// Hardness value.
    pub hardness: ArmorHardness,
    /// Armor type for matchup.
    pub armor_type: ArmorType,
    /// Remaining integrity.
    pub integrity: &'a mut ArmorIntegrity,
}

/// Whether the struck piece still has integrity left.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Protecting(bool);

impl Protecting {
    /// Build from a bool.
    #[must_use]
    pub const fn new(protecting: bool) -> Self {
        Self(protecting)
    }
}

impl StruckPiece<'_> {
    /// True when integrity is still greater than zero.
    #[must_use]
    pub fn protects(&self) -> Protecting {
        Protecting::new(**self.integrity > 0)
    }

    /// Current integrity value.
    #[must_use]
    pub const fn integrity_value(&self) -> ArmorIntegrity {
        *self.integrity
    }
}

/// Mutable cover and slab ledgers that a shot may damage.
pub struct StruckSurfaces<'a> {
    /// Cover ledger.
    pub cover: &'a mut CoverLedger,
    /// Slab ledger.
    pub slab: &'a mut SlabLedger,
}

/// What the hit actually did.
#[derive(Debug, Clone, PartialEq)]
pub enum HitVerdict {
    /// Applied to a combatant.
    Ganger(Box<GangerVerdict>),
    /// Applied to cover.
    Cover(CoverVerdict),
    /// Applied to a slab.
    Slab(SlabVerdict),
    /// Applied to the ground.
    Ground(GroundAccrual),
    /// No meaningful effect.
    NoEffect,
}

impl HitVerdict {
    /// Entity of the ganger that was struck, if any.
    #[must_use]
    pub fn struck_ganger(&self) -> Option<Entity> {
        match self {
            Self::Ganger(verdict) => Some(verdict.target),
            Self::Cover(_) | Self::Slab(_) | Self::Ground(_) | Self::NoEffect => None,
        }
    }
}

/// Full report for one resolved shot.
#[derive(Debug, Clone, PartialEq)]
pub struct HitReport {
    /// Original shot kind.
    pub kind: ShotKind,
    /// What happened.
    pub verdict: HitVerdict,
}

impl HitReport {
    /// Report with no effect.
    #[must_use]
    pub const fn no_effect(kind: ShotKind) -> Self {
        Self {
            kind,
            verdict: HitVerdict::NoEffect,
        }
    }
}
