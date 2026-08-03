//! Reduce integrity; emit damaged or broken outcomes.

use bevy::prelude::{Entity, Message};

use crate::{
    armor::{ArmorIntegrity, BodyPart},
    resolve_hit::IntegrityWear,
};

/// Armor took integrity damage but is still protecting.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArmorDamaged {
    /// Wearer.
    pub ganger: Entity,
    /// Body part.
    pub part: BodyPart,
    /// Integrity removed.
    pub delta: IntegrityWear,
}

impl ArmorDamaged {
    /// Build a damage message.
    #[must_use]
    pub const fn new(ganger: Entity, part: BodyPart, delta: IntegrityWear) -> Self {
        Self {
            ganger,
            part,
            delta,
        }
    }
}

/// Result of applying wear to a piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArmorWearOutcome {
    /// No change (already broken or zero wear).
    Unaffected,
    /// Still protecting after damage.
    Damaged(ArmorDamaged),
    /// Crossed from protecting to broken.
    Broke(ArmorBroken),
}

/// Armor piece integrity hit zero.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArmorBroken {
    /// Wearer.
    pub ganger: Entity,
    /// Body part.
    pub part: BodyPart,
}

impl ArmorBroken {
    /// Build a break message.
    #[must_use]
    pub const fn new(ganger: Entity, part: BodyPart) -> Self {
        Self { ganger, part }
    }
}

/// Subtract wear from integrity; return the outcome for messaging.
#[must_use]
pub fn wear_armor(
    integrity: &mut ArmorIntegrity,
    part: BodyPart,
    wear: IntegrityWear,
    ganger: Entity,
) -> ArmorWearOutcome {
    let was_protecting = **integrity > 0;

    *integrity = ArmorIntegrity::new(**integrity - *wear);

    let now_broken = **integrity <= 0;

    if was_protecting && now_broken {
        ArmorWearOutcome::Broke(ArmorBroken::new(ganger, part))
    } else if was_protecting && *wear > 0 {
        ArmorWearOutcome::Damaged(ArmorDamaged::new(ganger, part, wear))
    } else {
        ArmorWearOutcome::Unaffected
    }
}
