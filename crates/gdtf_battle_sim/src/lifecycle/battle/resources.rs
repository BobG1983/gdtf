//! Live battle state resources.

use bevy::{
    platform::collections::HashSet,
    prelude::{Deref, Resource},
};

use crate::ganger::Faction;

/// Marker: a battle is currently in progress.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BattleInProgress;

/// Faction controlled by the player.
#[derive(Resource, Deref, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerFaction(Faction);

impl PlayerFaction {
    /// Wrap a faction index.
    #[must_use]
    pub const fn new(faction: Faction) -> Self {
        Self(faction)
    }
}

/// Factions present on the battlefield.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub struct BattleRoster(HashSet<Faction>);

impl BattleRoster {
    /// From an iterator of factions.
    #[must_use]
    pub fn new(factions: impl IntoIterator<Item = Faction>) -> Self {
        Self(factions.into_iter().collect())
    }

    /// Every faction fielded in this battle, in no particular order.
    pub fn factions(&self) -> impl Iterator<Item = Faction> + '_ {
        self.0.iter().copied()
    }

    /// Whether the player faction is present.
    #[must_use]
    pub fn has_player(&self, player: Faction) -> PlayersFielded {
        PlayersFielded::new(self.0.contains(&player))
    }

    /// Whether any non-player faction is present.
    #[must_use]
    pub fn has_enemy_of(&self, player: Faction) -> EnemiesFielded {
        EnemiesFielded::new(self.0.iter().any(|&faction| faction != player))
    }
}

/// Whether the player side has units fielded.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayersFielded(bool);

impl PlayersFielded {
    /// Wrap the flag.
    #[must_use]
    pub const fn new(fielded: bool) -> Self {
        Self(fielded)
    }
}

/// Whether any enemy side has units fielded.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnemiesFielded(bool);

impl EnemiesFielded {
    /// Wrap the flag.
    #[must_use]
    pub const fn new(fielded: bool) -> Self {
        Self(fielded)
    }
}
