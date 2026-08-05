//! System-param bundles for act writers, the shown level, and selection cycling.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_presenter::{ActiveLevel, ViewMode};
use gdtf_battle_sim::{
    acts::{
        EndTurnRequested, FireRequested, MoveRequested, ReloadRequested, SetAimingRequested,
        SetFacingRequested, SetStanceRequested,
    },
    battle::PlayerFaction,
    ganger::LifeState,
    prelude::{Faction, Position},
};

use crate::{
    SelectedShooter,
    intent::level::{LevelStep, step_level},
    selection::cell_order_key,
};

/// Message writers for sim act requests.
#[derive(SystemParam)]
pub struct ActWriters<'w> {
    pub(super) fire:     MessageWriter<'w, FireRequested>,
    pub(super) movement: MessageWriter<'w, MoveRequested>,
    pub(super) stance:   MessageWriter<'w, SetStanceRequested>,
    pub(super) aiming:   MessageWriter<'w, SetAimingRequested>,
    pub(super) facing:   MessageWriter<'w, SetFacingRequested>,
    pub(super) reload:   MessageWriter<'w, ReloadRequested>,
    pub(super) end_turn: MessageWriter<'w, EndTurnRequested>,
}

/// The battlescape slice on screen: active level and single/full view mode.
#[derive(SystemParam)]
pub struct ShownLevel<'w> {
    active: ResMut<'w, ActiveLevel>,
    mode:   ResMut<'w, ViewMode>,
}

impl ShownLevel<'_> {
    /// Move the active level one storey up or down.
    pub(super) fn step(&mut self, direction: LevelStep) {
        let next = step_level(**self.active, direction);
        if next != **self.active {
            *self.active = ActiveLevel::new(next);
        }
    }

    /// Flip between single-level and all-levels view.
    pub(super) fn toggle_full_view(&mut self) {
        let flipped = self.mode.toggled();
        *self.mode = flipped;
    }
}

/// Queries used when cycling or validating player selection.
#[derive(SystemParam)]
pub struct SelectionCycleReads<'w, 's> {
    player:   Option<Res<'w, PlayerFaction>>,
    gangers:  Query<'w, 's, (Entity, &'static Faction, &'static Position)>,
    factions: Query<'w, 's, &'static Faction>,
    lifes:    Query<'w, 's, &'static LifeState>,
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
