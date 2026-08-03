use bevy::prelude::*;
use gdtf_battle_sim::{
    battle::PlayerFaction,
    ganger::LifeState,
    prelude::{Faction, Position},
};

use crate::selection::{
    order::cell_order_key,
    resources::{SelectedShooter, set_selection},
};

pub fn auto_select_first_player_ganger(
    gangers: Query<(Entity, &Faction, &Position, Option<&LifeState>)>,
    player: Res<PlayerFaction>,
    mut selected: ResMut<SelectedShooter>,
) {
    if selected.is_some() {
        return;
    }
    let player_faction = **player;
    let pick = gangers
        .iter()
        .filter(|(_, faction, _, life)| {
            **faction == player_faction && life.is_none_or(|life| *life.is_active())
        })
        .min_by_key(|(_, _, position, _)| cell_order_key(position))
        .map(|(entity, ..)| entity);
    if let Some(entity) = pick {
        set_selection(&mut selected, SelectedShooter::new(entity));
    }
}

pub fn clear_downed_selection(lifes: Query<&LifeState>, mut selected: ResMut<SelectedShooter>) {
    let Some(actor) = **selected else {
        return; 
    };
    if let Ok(life) = lifes.get(actor)
        && !*life.is_active()
    {
        *selected = SelectedShooter::cleared();
    }
}
