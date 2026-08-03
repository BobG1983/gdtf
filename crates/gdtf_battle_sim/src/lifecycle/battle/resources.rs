use bevy::{
    platform::collections::HashSet,
    prelude::{Deref, Resource},
};

use crate::ganger::Faction;

#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BattleInProgress;

#[derive(Resource, Deref, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerFaction(Faction);

impl PlayerFaction {
                        #[must_use]
    pub const fn new(faction: Faction) -> Self {
        Self(faction)
    }
}

#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub struct BattleRoster(HashSet<Faction>);

impl BattleRoster {
                        #[must_use]
    pub fn new(factions: impl IntoIterator<Item = Faction>) -> Self {
        Self(factions.into_iter().collect())
    }

            #[must_use]
    pub fn has_player(&self, player: Faction) -> PlayersFielded {
        PlayersFielded::new(self.0.contains(&player))
    }

                #[must_use]
    pub fn has_enemy_of(&self, player: Faction) -> EnemiesFielded {
        EnemiesFielded::new(self.0.iter().any(|&faction| faction != player))
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayersFielded(bool);

impl PlayersFielded {
        #[must_use]
    pub const fn new(fielded: bool) -> Self {
        Self(fielded)
    }
}

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnemiesFielded(bool);

impl EnemiesFielded {
        #[must_use]
    pub const fn new(fielded: bool) -> Self {
        Self(fielded)
    }
}
