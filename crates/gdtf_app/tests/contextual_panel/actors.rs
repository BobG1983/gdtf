use bevy::{ecs::entity::Entity, prelude::*};
use gdtf_app::test_support::{
    ContextualPanelRoot, ExecuteButton, MeleeButton, ShoveButton, StabilizeButton,
};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    effects::bleed::BleedingOut,
    prelude::{Cell, CellLevel, Faction, Level, LifeState, Position, Stance, StanceKind},
};

use super::harness::*;

pub(crate) fn at(x: i32, y: i32) -> Position {
    Position::new(CellLevel::new(Cell::new(x, y), Level::new(0)))
}

pub(crate) fn spawn_actor(app: &mut App, x: i32, y: i32, gang: u8) -> Entity {
    let actor = app.world_mut().spawn((at(x, y), Faction::new(gang))).id();
    app.world_mut().insert_resource(SelectedShooter::new(actor));
    actor
}

pub(crate) fn spawn_downed(
    app: &mut App,
    x: i32,
    y: i32,
    gang: u8,
    stabilized: Option<bool>,
) -> Entity {
    let mut entity = app
        .world_mut()
        .spawn((at(x, y), Faction::new(gang), LifeState::Downed));
    if stabilized != Some(true) {
        entity.insert(BleedingOut);
    }
    entity.id()
}

pub(crate) fn spawn_alive_enemy(app: &mut App, x: i32, y: i32, gang: u8) -> Entity {
    app.world_mut()
        .spawn((
            at(x, y),
            Faction::new(gang),
            LifeState::Alive,
            Stance::new(StanceKind::Standing),
        ))
        .id()
}

pub(crate) fn execute_visible(app: &mut App) -> bool {
    visibility::<ExecuteButton>(app) == Some(Visibility::Visible)
}

pub(crate) fn stabilize_visible(app: &mut App) -> bool {
    visibility::<StabilizeButton>(app) == Some(Visibility::Visible)
}

pub(crate) fn melee_visible(app: &mut App) -> bool {
    visibility::<MeleeButton>(app) == Some(Visibility::Visible)
}

pub(crate) fn shove_visible(app: &mut App) -> bool {
    visibility::<ShoveButton>(app) == Some(Visibility::Visible)
}

pub(crate) fn root_visible(app: &mut App) -> bool {
    visibility::<ContextualPanelRoot>(app) == Some(Visibility::Visible)
}
