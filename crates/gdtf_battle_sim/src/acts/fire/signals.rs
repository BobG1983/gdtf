use bevy::{
    ecs::system::SystemParam,
    prelude::{Deref, Entity, Message, MessageWriter, Query},
};

use crate::{
    acts::injury::InjuryInflicted,
    ganger::Position,
    occupancy_sync::{CoverDestroyed, GroundAccrued, SlabDestroyed},
    shot_fired::ShotFired,
    weapon::ModeKind,
};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct RoundCount(u32);

impl RoundCount {
        pub const NONE: Self = Self(0);

        #[must_use]
    pub const fn new(rounds: u32) -> Self {
        Self(rounds)
    }

                #[must_use]
    pub fn from_emitted(rounds: usize) -> Self {
        Self(u32::try_from(rounds).unwrap_or(u32::MAX))
    }
}

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FireDeclaration {
            pub shooter: Entity,
                    pub target:  Option<Entity>,
                pub mode:    ModeKind,
                pub rounds:  RoundCount,
}

impl FireDeclaration {
                #[must_use]
    pub const fn new(
        shooter: Entity,
        target: Option<Entity>,
        mode: ModeKind,
        rounds: RoundCount,
    ) -> Self {
        Self {
            shooter,
            target,
            mode,
            rounds,
        }
    }
}

#[derive(SystemParam)]
pub struct FireSignals<'w, 's> {
            pub(super) shots:            MessageWriter<'w, ShotFired>,
            pub(super) declarations:     MessageWriter<'w, FireDeclaration>,
                        pub(super) cover_destroyed:  MessageWriter<'w, CoverDestroyed>,
                                pub(super) slab_destroyed:   MessageWriter<'w, SlabDestroyed>,
                                    pub(super) ground_accrued:   MessageWriter<'w, GroundAccrued>,
                            pub(super) injuries:         MessageWriter<'w, InjuryInflicted>,
                                pub(super) dots:             MessageWriter<'w, crate::effects::dot::DotApplied>,
                            pub(super) shoves:           MessageWriter<'w, crate::acts::request::ShoveRequested>,
                        pub(super) shove_tags:       Query<'w, 's, &'static crate::weapon::Shove>,
                                    pub(super) armor_breaks:     MessageWriter<'w, crate::armor_wear::ArmorBroken>,
                pub(super) deaths:           MessageWriter<'w, crate::effects::on_death::OnDeathOccurred>,
                    pub(super) ganger_positions: Query<'w, 's, &'static Position>,
}
