//! Act intent vocabulary pushed by input surfaces.

use bevy::prelude::Entity;
use gdtf_battle_sim::{
    acts::{AimRequest, FireRequested, MoveRequested, SetFacingRequested},
    prelude::StanceKind,
};

/// One player intent waiting to be dispatched into sim messages.
#[derive(Debug, Clone, PartialEq)]
pub enum ActIntent {
    /// Clear the current selection.
    SelectionClear,
    /// Raise the active view level.
    LevelUp,
    /// Lower the active view level.
    LevelDown,
    /// Toggle full-view mode.
    ToggleFullView,
    /// Cycle the selected shooter's stance.
    StanceCycle,
    /// Set stance to a specific kind.
    SetStance(StanceKind),
    /// Toggle aiming on the selected shooter.
    AimToggle,
    /// Set aiming on the selected shooter to a specific value.
    SetAiming(AimRequest),
    /// Cycle the selected shooter's facing.
    FacingCycle,
    /// Fire at a target.
    Fire(FireRequested),
    /// Move to a cell.
    Move(MoveRequested),
    /// Turn to face a direction.
    Turn(SetFacingRequested),
    /// Reload the selected shooter's weapon.
    Reload,
    /// End the current turn.
    EndTurn,
    /// Select the next player ganger.
    SelectNext,
    /// Select the previous player ganger.
    SelectPrev,
    /// Select a specific entity.
    Select(Entity),
}

impl ActIntent {
    /// Whether this intent requires the presenter to be caught up.
    #[must_use]
    pub const fn needs_caught_up(&self) -> bool {
        !matches!(self, Self::LevelUp | Self::LevelDown | Self::ToggleFullView)
    }
}
