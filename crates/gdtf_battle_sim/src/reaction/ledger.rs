//! Pending TU / facing / magazine overlays within one reaction pass.

use bevy::{platform::collections::HashMap, prelude::Entity};

use crate::{
    ganger::{Facing, Tu},
    magazine::Magazine,
    tu::spend_tu,
    weapon::ModeShots,
};

/// Predicted state after a successful interrupt (before ECS writeback).
pub(super) struct InterruptCommit {
    reactor:        Entity,
    weapon:         Entity,
    tu_after:       Tu,
    facing_after:   Facing,
    magazine_after: Magazine,
}

impl InterruptCommit {
    /// Project post-interrupt TU, facing, and magazine, or nothing when the pool falls short.
    #[must_use]
    pub(super) fn predict(
        reactor: Entity,
        weapon: Entity,
        tu_now: Tu,
        spend: Tu,
        facing_after: Facing,
        rounds: ModeShots,
        magazine: Magazine,
    ) -> Option<Self> {
        let mut tu_after = tu_now;
        spend_tu(&mut tu_after, spend).ok()?;
        let mut magazine_after = magazine;
        for _ in 0..*rounds {
            magazine_after.spend_round();
        }
        Some(Self {
            reactor,
            weapon,
            tu_after,
            facing_after,
            magazine_after,
        })
    }
}

struct ReactorOverlay {
    tu:     Tu,
    facing: Facing,
}

/// Tracks spends so multiple interrupts in one pass see updated state.
#[derive(Default)]
pub(super) struct PendingSpendLedger {
    overlays:  HashMap<Entity, ReactorOverlay>,
    magazines: HashMap<Entity, Magazine>,
}

impl PendingSpendLedger {
    #[must_use]
    pub(super) fn tu_of(&self, reactor: Entity, settled: Tu) -> Tu {
        self.overlays
            .get(&reactor)
            .map_or(settled, |overlay| overlay.tu)
    }

    #[must_use]
    pub(super) fn facing_of(&self, reactor: Entity, settled: Facing) -> Facing {
        self.overlays
            .get(&reactor)
            .map_or(settled, |overlay| overlay.facing)
    }

    #[must_use]
    pub(super) fn magazine_of(&self, weapon: Entity, live: Magazine) -> Magazine {
        self.magazines
            .get(&weapon)
            .map_or(live, |magazine| *magazine)
    }

    pub(super) fn commit(&mut self, commit: InterruptCommit) {
        self.overlays.insert(
            commit.reactor,
            ReactorOverlay {
                tu:     commit.tu_after,
                facing: commit.facing_after,
            },
        );
        self.magazines.insert(commit.weapon, commit.magazine_after);
    }
}
