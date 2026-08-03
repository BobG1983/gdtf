use bevy::{prelude::*, window::PrimaryWindow};
use gdtf_battle_presenter::ActiveLevel;
use gdtf_battle_sim::prelude::{CellLevel, Level};

use crate::{
    gamepad::{ActivePointer, GamepadCursor},
    picking::projection::world_to_cell,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectMode {
        Hovered(Option<CellLevel>),
        Pinned(CellLevel),
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InspectTarget {
        hovered: Option<CellLevel>,
            pinned:  Option<CellLevel>,
}

impl InspectTarget {
                        #[must_use]
    pub const fn new(hovered: Option<CellLevel>) -> Self {
        Self {
            hovered,
            pinned: None,
        }
    }

                    #[must_use]
    pub const fn hovered(&self) -> Option<CellLevel> {
        self.hovered
    }

            pub const fn set_hovered(&mut self, cell: Option<CellLevel>) {
        self.hovered = cell;
    }

                    #[must_use]
    pub const fn pinned(&self) -> Option<CellLevel> {
        self.pinned
    }

                pub const fn set_pinned(&mut self, cell: CellLevel) {
        self.pinned = Some(cell);
    }

            pub const fn clear_pin(&mut self) {
        self.pinned = None;
    }

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
        node:       &'static ComputedNode,
        transform:  &'static UiGlobalTransform,
        visibility: &'static InheritedVisibility,
}

fn cursor_over_ui(ui_nodes: &Query<UiNodeHit>, cursor: Vec2) -> bool {
    ui_nodes
        .iter()
        .any(|hit| hit.visibility.get() && hit.node.contains_point(*hit.transform, cursor))
}

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
