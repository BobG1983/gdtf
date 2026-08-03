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

pub struct TargetGanger<'a> {
        pub hp:        &'a mut Hp,
        pub wounds:    &'a mut Wounds,
        pub life:      &'a mut LifeState,
                        pub piece:     Option<StruckPiece<'a>>,
            pub inflicted: &'a mut InflictedWounds,
        pub toughness: Toughness,
        pub luck:      Luck,
}

pub struct StruckPiece<'a> {
        pub floor:      ArmorFloor,
        pub protection: ArmorProtection,
        pub hardness:   ArmorHardness,
        pub armor_type: ArmorType,
        pub integrity:  &'a mut ArmorIntegrity,
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Protecting(bool);

impl Protecting {
        #[must_use]
    pub const fn new(protecting: bool) -> Self {
        Self(protecting)
    }
}

impl StruckPiece<'_> {
                    #[must_use]
    pub fn protects(&self) -> Protecting {
        Protecting::new(**self.integrity > 0)
    }

                #[must_use]
    pub const fn integrity_value(&self) -> ArmorIntegrity {
        *self.integrity
    }
}

pub struct StruckSurfaces<'a> {
            pub cover: &'a mut CoverLedger,
            pub slab:  &'a mut SlabLedger,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HitVerdict {
        Ganger(Box<GangerVerdict>),
        Cover(CoverVerdict),
        Slab(SlabVerdict),
        Ground(GroundAccrual),
        NoEffect,
}

impl HitVerdict {
                                    #[must_use]
    pub fn struck_ganger(&self) -> Option<Entity> {
        match self {
            Self::Ganger(verdict) => Some(verdict.target),
            Self::Cover(_) | Self::Slab(_) | Self::Ground(_) | Self::NoEffect => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HitReport {
        pub kind:    ShotKind,
        pub verdict: HitVerdict,
}

impl HitReport {
                                #[must_use]
    pub const fn no_effect(kind: ShotKind) -> Self {
        Self {
            kind,
            verdict: HitVerdict::NoEffect,
        }
    }
}
