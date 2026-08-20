use bevy::{prelude::*, ui::Display};
use gdtf_battle_input::{InputSystems, InspectTarget, pick_hovered_cell};
use gdtf_battle_sim::{prelude::CellLevel, visibility::SquadVisibility};

use super::harness::*;

/// Frames the screen gets to play what a fog change logged before it draws the new fog.
const CATCH_UP_FRAMES: u8 = 64;

// ---------------------------------------------------------------------------------

#[derive(Resource, Clone, Copy, Default)]
pub(crate) struct DesiredHover(Option<CellLevel>);

pub(crate) fn force_hover(desired: Res<DesiredHover>, mut target: ResMut<InspectTarget>) {
    target.set_hovered(desired.0);
}

pub(crate) fn hover_app() -> App {
    let mut app = battle_running_app();
    app.world_mut().insert_resource(DesiredHover::default());
    app.add_systems(
        Update,
        force_hover
            .in_set(InputSystems::Gather)
            .after(pick_hovered_cell),
    );
    app
}

pub(crate) fn hover(app: &mut App, cell: Option<CellLevel>) {
    app.world_mut().insert_resource(DesiredHover(cell));
    app.update();
}

pub(crate) fn display_of<M: Component>(app: &mut App) -> Option<Display> {
    let entity = single_global::<M>(app)?;
    app.world().get::<Node>(entity).map(|n| n.display)
}

pub(crate) fn make_cells_visible(app: &mut App, cells: &[CellLevel]) {
    let visible: bevy::platform::collections::HashSet<CellLevel> = cells.iter().copied().collect();
    let explored = visible.clone();
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible, explored));

    // A ganger coming into view logs an act, and the drawn fog waits for the screen to play it.
    for _ in 0..CATCH_UP_FRAMES {
        app.update();
        if screen_lights(app, cells) {
            return;
        }
    }
}

pub(crate) fn screen_lights(app: &App, cells: &[CellLevel]) -> bool {
    app.world()
        .get_resource::<gdtf_battle_presenter::ShownSquadVisibility>()
        .is_some_and(|shown| {
            cells
                .iter()
                .all(|at| *shown.visibility().is_cell_visible(at))
                && shown.visibility().visible_cells().count() == cells.len()
        })
}
