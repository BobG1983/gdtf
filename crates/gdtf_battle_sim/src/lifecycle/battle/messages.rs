use bevy::prelude::Message;

use crate::{rng::BattleSeed, situation::Situation};

#[derive(Message, Debug, Clone)]
pub struct SetupBattleRequested {
            pub situation: Situation,
                pub seed:      BattleSeed,
}

impl SetupBattleRequested {
        #[must_use]
    pub const fn new(situation: Situation, seed: BattleSeed) -> Self {
        Self { situation, seed }
    }
}

#[derive(Message, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct TeardownBattleRequested;

#[derive(Message, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct BattleReady;

#[derive(Message, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct BattleWon;

#[derive(Message, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct BattleLost;
