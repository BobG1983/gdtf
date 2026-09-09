//! Battle setup, teardown, and outcome messages.

use bevy::prelude::Message;

use crate::{
    rng::BattleSeed,
    situation::{PlacedGanger, Situation},
};

/// Request to set up a battle from a situation and the gangers deployed onto it.
#[derive(Message, Debug, Clone)]
pub struct SetupBattleRequested {
    /// Situation to spawn.
    pub situation:  Situation,
    /// Gangers deployed onto the situation's map, each at its spawn cell.
    pub placements: Vec<PlacedGanger>,
    /// RNG seed for this battle.
    pub seed:       BattleSeed,
}

impl SetupBattleRequested {
    /// Build the request.
    #[must_use]
    pub const fn new(
        situation: Situation,
        placements: Vec<PlacedGanger>,
        seed: BattleSeed,
    ) -> Self {
        Self {
            situation,
            placements,
            seed,
        }
    }
}

/// Request to tear down the current battle.
#[derive(Message, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct TeardownBattleRequested;

/// Battle setup finished successfully.
#[derive(Message, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct BattleReady;

/// Player won.
#[derive(Message, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct BattleWon;

/// Player lost.
#[derive(Message, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct BattleLost;
