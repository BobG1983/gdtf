//! What one generation resolved up front and its finish reads back.

use bevy::prelude::*;
use gdtf_battle_sim::{rng::BattleSeed, situation::Situation};

crate::support_item! {
    /// The authored situation and the root seed this generation runs from.
    #[derive(Resource, Debug, Clone)]
    struct BattleGenerationContext {
        authored: Situation,
        seed:     BattleSeed,
    }
}

impl BattleGenerationContext {
    /// Record what the generation about to run resolved.
    #[must_use]
    pub(super) const fn new(authored: Situation, seed: BattleSeed) -> Self {
        Self { authored, seed }
    }

    /// The authored situation generation started from.
    #[must_use]
    pub(super) const fn authored(&self) -> &Situation {
        &self.authored
    }

    /// The root seed this generation used.
    #[must_use]
    pub(super) const fn seed(&self) -> BattleSeed {
        self.seed
    }
}
