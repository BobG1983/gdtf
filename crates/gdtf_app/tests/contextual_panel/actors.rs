//! Shared live-slice actor spawners + cross-act visibility readers (used by every act file).

use bevy::{ecs::entity::Entity, prelude::*};
use gdtf_app::test_support::{
    ContextualPanelRoot, ExecuteButton, MeleeButton, ShoveButton, StabilizeButton,
};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    ganger::Stabilized,
    prelude::{Cell, CellLevel, Faction, Level, LifeState, Position, Stance, StanceKind},
};

use super::harness::*;

// ---------------------------------------------------------------------------------
// Live slice helpers — spawn an actor + downed neighbours, select, read targets, and
// drive a press through the real seam.
// ---------------------------------------------------------------------------------

/// A ground-level [`Position`] at cell `(x, y)`.
pub(crate) fn at(x: i32, y: i32) -> Position {
    Position::new(CellLevel::new(Cell::new(x, y), Level::new(0)))
}

/// Spawns the ACTOR — a ganger carrying exactly the components detection reads off the selection
/// (its [`Position`] + [`Faction`]) — at cell `(x, y)` in gang `gang`, and SELECTS it via the
/// [`SelectedShooter`] resource (the selection detection + the press router read). Returns its
/// entity.
pub(crate) fn spawn_actor(app: &mut App, x: i32, y: i32, gang: u8) -> Entity {
    let actor = app.world_mut().spawn((at(x, y), Faction::new(gang))).id();
    app.world_mut().insert_resource(SelectedShooter::new(actor));
    actor
}

/// Spawns a DOWNED neighbour at cell `(x, y)` in gang `gang` — [`LifeState::Downed`] plus its
/// [`Position`] / [`Faction`], and (when `stabilized` is `Some`) a [`Stabilized`] flag. Returns
/// its entity. The detection scan reads exactly these components.
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
    if let Some(flag) = stabilized {
        entity.insert(Stabilized::new(flag));
    }
    entity.id()
}

/// Spawns an ALIVE enemy at cell `(x, y)` in gang `gang` — [`LifeState::Alive`] plus its
/// [`Position`] / [`Faction`] / [`Stance`] (the melee detection scan + the LOS aim silhouette
/// read exactly these). Returns its entity (GTW-507).
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

/// The per-act `ContextualOffer` seams are not directly readable across the crate boundary
/// (their contents are private), so detection coverage reads the panel's observable effects —
/// the per-button [`Visibility`] — and the press tests read the emitted `*Requested` (e.g.
/// [`ExecuteDownedRequested`]). This helper reads the Execute button visibility.
pub(crate) fn execute_visible(app: &mut App) -> bool {
    visibility::<ExecuteButton>(app) == Some(Visibility::Visible)
}

/// Reads whether the Stabilize button is visible.
pub(crate) fn stabilize_visible(app: &mut App) -> bool {
    visibility::<StabilizeButton>(app) == Some(Visibility::Visible)
}

/// Reads whether the Melee button is visible (GTW-507).
pub(crate) fn melee_visible(app: &mut App) -> bool {
    visibility::<MeleeButton>(app) == Some(Visibility::Visible)
}

/// Reads whether the Shove button is visible (GTW-525).
pub(crate) fn shove_visible(app: &mut App) -> bool {
    visibility::<ShoveButton>(app) == Some(Visibility::Visible)
}

/// Reads whether the panel root is visible.
pub(crate) fn root_visible(app: &mut App) -> bool {
    visibility::<ContextualPanelRoot>(app) == Some(Visibility::Visible)
}
