use bevy::{prelude::*, ui::Display};
use gdtf_battle_input::{InputSystems, InspectTarget, pick_hovered_cell};
use gdtf_battle_sim::prelude::CellLevel;

use super::harness::*;

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
