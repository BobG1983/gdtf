//! Inspect target hover and pin state.

use bevy::{prelude::*, window::PrimaryWindow};
use gdtf_battle_presenter::ActiveLevel;
use gdtf_battle_sim::prelude::{CellLevel, Level};

use crate::{
    gamepad::{ActivePointer, GamepadCursor},
    picking::projection::world_to_cell,
};

/// Whether the inspect target is free-hover or pinned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectMode {
    /// Follow the pointer; may be empty over UI or off-map.
    Hovered(Option<CellLevel>),
    /// Locked to a cell until cleared.
    Pinned(CellLevel),
}

/// Hovered and pinned cells under the pointer.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InspectTarget {
    hovered: Option<CellLevel>,
    pinned: Option<CellLevel>,
}

impl InspectTarget {
    /// Start with a hover cell and no pin.
    #[must_use]
    pub const fn new(hovered: Option<CellLevel>) -> Self {
        Self {
            hovered,
            pinned: None,
        }
    }

    /// Current hover cell.
    #[must_use]
    pub const fn hovered(&self) -> Option<CellLevel> {
        self.hovered
    }

    /// Set the hover cell.
    pub const fn set_hovered(&mut self, cell: Option<CellLevel>) {
        self.hovered = cell;
    }

    /// Current pin cell.
    #[must_use]
    pub const fn pinned(&self) -> Option<CellLevel> {
        self.pinned
    }

    /// Pin a cell.
    pub const fn set_pinned(&mut self, cell: CellLevel) {
        self.pinned = Some(cell);
    }

    /// Clear the pin.
    pub const fn clear_pin(&mut self) {
        self.pinned = None;
    }

    /// Effective inspect mode (pin wins over hover).
    #[must_use]
    pub const fn effective(&self) -> InspectMode {
        match self.pinned {
            Some(cell) => InspectMode::Pinned(cell),
            None => InspectMode::Hovered(self.hovered),
        }
    }
}

#[derive(bevy::ecs::query::QueryData)]
pub struct UiNodeHit {
    node: &'static ComputedNode,
    transform: &'static UiGlobalTransform,
    visibility: &'static InheritedVisibility,
}

fn cursor_over_ui(ui_nodes: &Query<UiNodeHit>, cursor: Vec2) -> bool {
    ui_nodes
        .iter()
        .any(|hit| hit.visibility.get() && hit.node.contains_point(*hit.transform, cursor))
}

/// Update hover cell from mouse or gamepad cursor, skipping UI hits.
pub fn pick_hovered_cell(
    cameras: Query<(&Camera, &GlobalTransform), With<gdtf_battle_presenter::WorldCamera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    ui_nodes: Query<UiNodeHit>,
    active_level: Res<ActiveLevel>,
    active: Res<ActivePointer>,
    gamepad_cursor: Res<GamepadCursor>,
    mut target: ResMut<InspectTarget>,
) {
    let resolved = resolve_hovered_cell(
        &cameras,
        &windows,
        &ui_nodes,
        **active_level,
        *active,
        *gamepad_cursor,
    );
    if target.hovered() != resolved {
        target.set_hovered(resolved);
    }
}

fn active_cursor(
    window: &Window,
    active: ActivePointer,
    gamepad_cursor: GamepadCursor,
) -> Option<Vec2> {
    match active {
        ActivePointer::Mouse => window.cursor_position(),
        ActivePointer::Gamepad => Some(*gamepad_cursor),
    }
}

fn resolve_hovered_cell(
    cameras: &Query<(&Camera, &GlobalTransform), With<gdtf_battle_presenter::WorldCamera>>,
    windows: &Query<&Window, With<PrimaryWindow>>,
    ui_nodes: &Query<UiNodeHit>,
    level: Level,
    active: ActivePointer,
    gamepad_cursor: GamepadCursor,
) -> Option<CellLevel> {
    let Ok((camera, cam_transform)) = cameras.single() else {
        return None;
    };
    let Ok(window) = windows.single() else {
        return None;
    };
    let cursor = active_cursor(window, active, gamepad_cursor)?;
    if cursor_over_ui(ui_nodes, cursor * window.scale_factor()) {
        return None;
    }
    let viewport = camera.logical_viewport_rect()?;
    if !viewport.contains(cursor) {
        return None;
    }
    let Ok(world) = camera.viewport_to_world_2d(cam_transform, cursor) else {
        return None;
    };
    world_to_cell(world, level)
}
