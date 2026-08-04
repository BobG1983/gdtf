//! Battle setup, teardown, and outcome messages.

use bevy::prelude::Message;

use crate::{rng::BattleSeed, situation::Situation};

/// Request to set up a battle from a situation.
#[derive(Message, Debug, Clone)]
pub struct SetupBattleRequested {
    /// Situation to spawn.
    pub situation: Situation,
    /// RNG seed for this battle.
    pub seed:      BattleSeed,
}

impl SetupBattleRequested {
    /// Build the request.
    #[must_use]
    pub const fn new(situation: Situation, seed: BattleSeed) -> Self {
        Self { situation, seed }
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
