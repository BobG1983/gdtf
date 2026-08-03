//! System-param bundles for left-click and turn decisions.

use bevy::prelude::*;
use gdtf_battle_sim::{
    battle::PlayerFaction, prelude::OccupancyGrid, tuning::CombatTuning,
    vertical::VerticalLinkGraph, visibility::SquadVisibility,
};

use crate::{InspectTarget, SelectedFireMode, selection::SelectedShooter};

/// Resources read by left-click decision and systems.
#[derive(bevy::ecs::system::SystemParam)]
pub struct LeftClickReads<'w> {
    pub(in crate::pointer::selection) mouse: Res<'w, ButtonInput<MouseButton>>,
    pub(super) occupancy: Res<'w, OccupancyGrid>,
    pub(super) fire_mode: Res<'w, SelectedFireMode>,
    pub(super) tuning: Res<'w, CombatTuning>,
    pub(super) player: Res<'w, PlayerFaction>,
    pub(super) links: Res<'w, VerticalLinkGraph>,
    pub(super) squad_visibility: Option<Res<'w, SquadVisibility>>,
}

/// Resources read by right-click turn systems.
#[derive(bevy::ecs::system::SystemParam)]
pub struct TurnReads<'w> {
    /// Current inspect / hover target.
    pub hovered: Res<'w, InspectTarget>,
    /// Player faction.
    pub player: Res<'w, PlayerFaction>,
    /// Selected shooter.
    pub selected: Res<'w, SelectedShooter>,
}
