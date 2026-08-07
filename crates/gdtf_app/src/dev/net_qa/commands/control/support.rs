//! Shared reads for the view and battle control commands.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_input::{ActIntent, LevelStep, PendingActIntent, step_level, world_to_cell};
use gdtf_battle_presenter::{ActiveLevel, ViewMode, WorldCamera};
use gdtf_qa_protocol::command::RefusalNote;

use crate::dev::net_qa::wire::{
    cell::{CellLevelNet, LevelNet},
    misc::ViewModeNet,
};

/// The refusal a view command gets when the battlescape view resources are not up.
pub(super) const NO_VIEW: RefusalNote = RefusalNote::from_static(
    "the battlescape view resources are not up, so there is no storey or view mode to step",
);

/// The refusal a camera command gets when no world camera is on the screen.
pub(super) const NO_CAMERA: RefusalNote = RefusalNote::from_static(
    "the world camera is spawned with the battle screen and despawned with it, and no camera is \
     on the screen right now",
);

/// Which of the three view intents a command pushes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ViewChange {
    /// Step the active storey one up or down.
    Level(LevelStep),
    /// Flip between single-level and all-levels view.
    FullView,
}

/// Where a view command leaves the battlescape slice on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ShownView {
    /// Storey the view is on.
    pub(super) level:     LevelNet,
    /// Whether every storey is drawn.
    pub(super) full_view: ViewModeNet,
}

/// The act bus and the view resources the three view commands read back from.
#[derive(SystemParam)]
pub(super) struct ViewControls<'w> {
    active:  Option<Res<'w, ActiveLevel>>,
    mode:    Option<Res<'w, ViewMode>>,
    pending: Option<ResMut<'w, PendingActIntent>>,
}

impl ViewControls<'_> {
    /// Push `change` onto the act bus and report where it leaves the view.
    pub(super) fn drive(&mut self, change: ViewChange) -> Option<ShownView> {
        let active = **self.active.as_deref()?;
        let mode = *self.mode.as_deref()?;
        let pending = self.pending.as_mut()?;
        let shown = match change {
            ViewChange::Level(direction) => {
                pending.push(level_intent(direction));
                ShownView {
                    level:     LevelNet::new(*step_level(active, direction)),
                    full_view: ViewModeNet::from_view(mode),
                }
            }
            ViewChange::FullView => {
                pending.push(ActIntent::ToggleFullView);
                ShownView {
                    level:     LevelNet::new(*active),
                    full_view: ViewModeNet::from_view(mode.toggled()),
                }
            }
        };
        Some(shown)
    }
}

const fn level_intent(direction: LevelStep) -> ActIntent {
    match direction {
        LevelStep::Up => ActIntent::LevelUp,
        LevelStep::Down => ActIntent::LevelDown,
    }
}

/// The cell the world camera is centred on, or nothing when it sits off the grid.
pub(super) fn camera_cell(
    active: Option<&ActiveLevel>,
    cameras: &Query<&Transform, With<WorldCamera>>,
) -> Option<CellLevelNet> {
    let level = **active?;
    let transform = cameras.iter().next()?;
    let centre = Vec2::new(transform.translation.x, transform.translation.y);
    world_to_cell(centre, level).map(CellLevelNet::from_sim)
}
