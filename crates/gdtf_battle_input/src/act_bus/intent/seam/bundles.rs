//! System-param bundles for act writers and selection cycling.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_sim::{
    acts::{
        EndTurnRequested, FireRequested, MoveRequested, ReloadRequested, SetAimingRequested,
        SetFacingRequested, SetStanceRequested,
    },
    battle::PlayerFaction,
    ganger::LifeState,
    prelude::{Faction, Position},
};

use crate::{SelectedShooter, selection::cell_order_key};

/// Message writers for sim act requests.
#[derive(SystemParam)]
pub struct ActWriters<'w> {
    pub(super) fire: MessageWriter<'w, FireRequested>,
    pub(super) movement: MessageWriter<'w, MoveRequested>,
    pub(super) stance: MessageWriter<'w, SetStanceRequested>,
    pub(super) aiming: MessageWriter<'w, SetAimingRequested>,
    pub(super) facing: MessageWriter<'w, SetFacingRequested>,
    pub(super) reload: MessageWriter<'w, ReloadRequested>,
    pub(super) end_turn: MessageWriter<'w, EndTurnRequested>,
}

/// Queries used when cycling or validating player selection.
#[derive(SystemParam)]
pub struct SelectionCycleReads<'w, 's> {
    player: Option<Res<'w, PlayerFaction>>,
    gangers: Query<'w, 's, (Entity, &'static Faction, &'static Position)>,
    factions: Query<'w, 's, &'static Faction>,
    lifes: Query<'w, 's, &'static LifeState>,
}

impl SelectionCycleReads<'_, '_> {
    pub(super) fn ordered_player_gangers(&self) -> Vec<Entity> {
        let Some(player) = self.player.as_ref() else {
            return Vec::new();
        };
        let player_faction: Faction = ***player;
        let mut ordered: Vec<(Entity, Position)> = self
            .gangers
            .iter()
            .filter(|(entity, faction, _)| **faction == player_faction && self.is_alive(*entity))
            .map(|(entity, _, position)| (entity, *position))
            .collect();
        ordered.sort_by_key(|(_, position)| cell_order_key(position));
        ordered.into_iter().map(|(entity, _)| entity).collect()
    }

    pub(super) fn select_target(&self, entity: Entity) -> Option<SelectedShooter> {
        let player_faction: Faction = ***self.player.as_ref()?;
        let faction = self.factions.get(entity).ok()?;
        (*faction == player_faction && self.is_alive(entity)).then(|| SelectedShooter::new(entity))
    }

    fn is_alive(&self, entity: Entity) -> bool {
        match self.lifes.get(entity) {
            Ok(life) => *life.is_active(),
            Err(_) => true,
        }
    }
}
