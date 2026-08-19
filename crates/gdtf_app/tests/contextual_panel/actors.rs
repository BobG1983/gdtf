use bevy::{ecs::entity::Entity, prelude::*};
use gdtf_app::test_support::{
    ContextualPanelRoot, EnterEmplacementButton, ExecuteButton, ExitEmplacementButton, MeleeButton,
    ShoveButton, StabilizeButton,
};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    acts::{
        downed::{execute_tu_cost, stabilize_tu_cost},
        shove_tu_cost,
    },
    effects::bleed::BleedingOut,
    ganger::Tu,
    magazine::{LoadedRounds, Magazine, ReloadTu},
    prelude::{Cell, CellLevel, Faction, Level, LifeState, Position, Stance, StanceKind},
    tuning::CombatTuning,
    weapon::MagazineSize,
};

use super::harness::*;

pub(crate) fn at(x: i32, y: i32) -> Position {
    Position::new(CellLevel::new(Cell::new(x, y), Level::new(0)))
}

/// What one shove would charge, taken from the sim's own cost helper against the live tuning.
pub(crate) fn shove_cost(app: &App) -> Tu {
    app.world()
        .get_resource::<CombatTuning>()
        .map_or_else(|| Tu::new(0), shove_tu_cost)
}

/// What one execute would charge, taken from the sim's own cost helper against the live tuning.
pub(crate) fn execute_cost(app: &App) -> Tu {
    app.world()
        .get_resource::<CombatTuning>()
        .map_or_else(|| Tu::new(0), execute_tu_cost)
}

/// What one stabilize would charge, taken from the sim's own cost helper against the live tuning.
pub(crate) fn stabilize_cost(app: &App) -> Tu {
    app.world()
        .get_resource::<CombatTuning>()
        .map_or_else(|| Tu::new(0), stabilize_tu_cost)
}

/// A magazine holding `rounds` of a two-round capacity.
pub(crate) const fn magazine_of(rounds: u16) -> Magazine {
    Magazine::new(
        LoadedRounds::new(rounds),
        MagazineSize::new(2),
        ReloadTu::new(1),
    )
}

pub(crate) fn spawn_actor(app: &mut App, x: i32, y: i32, gang: u8) -> Entity {
    let pool = shove_cost(app);
    let actor = app
        .world_mut()
        .spawn((at(x, y), Faction::new(gang), LifeState::Alive, pool))
        .id();
    app.world_mut().insert_resource(SelectedShooter::new(actor));
    actor
}

/// Overwrite a spawned ganger's life state.
pub(crate) fn set_life(app: &mut App, ganger: Entity, life: LifeState) {
    if let Ok(mut entity) = app.world_mut().get_entity_mut(ganger) {
        entity.insert(life);
    }
}

/// Move a spawned ganger to another cell on level 0.
pub(crate) fn move_to(app: &mut App, ganger: Entity, x: i32, y: i32) {
    if let Some(mut position) = app.world_mut().get_mut::<Position>(ganger) {
        *position = at(x, y);
    }
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

pub(crate) fn enter_emplacement_visible(app: &mut App) -> bool {
    visibility::<EnterEmplacementButton>(app) == Some(Visibility::Visible)
}

pub(crate) fn exit_emplacement_visible(app: &mut App) -> bool {
    visibility::<ExitEmplacementButton>(app) == Some(Visibility::Visible)
}

pub(crate) fn root_visible(app: &mut App) -> bool {
    visibility::<ContextualPanelRoot>(app) == Some(Visibility::Visible)
}
