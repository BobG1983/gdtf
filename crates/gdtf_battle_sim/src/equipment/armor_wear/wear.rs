use bevy::prelude::{Entity, Message};

use crate::{
    armor::{ArmorIntegrity, BodyPart},
    resolve_hit::IntegrityWear,
};

/// A buffered Bevy **message** (`#[derive(Message)]`), mirroring [`ArmorBroken`] /
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArmorDamaged {
        pub ganger: Entity,
        pub part:   BodyPart,
        pub delta:  IntegrityWear,
}

impl ArmorDamaged {
            #[must_use]
    pub const fn new(ganger: Entity, part: BodyPart, delta: IntegrityWear) -> Self {
        Self {
            ganger,
            part,
            delta,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArmorWearOutcome {
        Unaffected,
        Damaged(ArmorDamaged),
        Broke(ArmorBroken),
}

/// A buffered Bevy **message** (`#[derive(Message)]`), mirroring
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArmorBroken {
        pub ganger: Entity,
        pub part:   BodyPart,
}

impl ArmorBroken {
            #[must_use]
    pub const fn new(ganger: Entity, part: BodyPart) -> Self {
        Self { ganger, part }
    }
}

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
