//! Shared hover-driving vocabulary for both hover-inspect files.

use bevy::{prelude::*, ui::Display};
use gdtf_battle_input::{InputSystems, InspectTarget, pick_hovered_cell};
use gdtf_battle_sim::prelude::CellLevel;

use super::harness::*;

// ---------------------------------------------------------------------------------
// AC2 (hover) — the inspect panel inspects what the cursor hovers.
// ---------------------------------------------------------------------------------

/// A test-controlled desired hover cell, copied into `InspectTarget` AFTER the headless picker
/// runs (which would otherwise clobber an injected value to `None`).
#[derive(Resource, Clone, Copy, Default)]
pub(crate) struct DesiredHover(Option<CellLevel>);

/// Copies [`DesiredHover`] into [`InspectTarget`]'s live hovered cell — registered
/// `.after(pick_hovered_cell)` in `InputSystems::Gather`, so it is the LAST hovered-cell writer
/// of the frame and the `.after(Gather)` inspect-panel update reads it. Writes ONLY the hovered
/// cell (via `set_hovered`), leaving any pin untouched (GTW-300). The cursor→cell pick is the one
/// external this stubs (it is tested in `gdtf_battle_input`); everything downstream is the real
/// path.
pub(crate) fn force_hover(desired: Res<DesiredHover>, mut target: ResMut<InspectTarget>) {
    target.set_hovered(desired.0);
}

/// Builds the battle app with the hover-forcing system wired in.
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

/// Sets the desired hover cell and steps one update so the real `update_inspect_panel` reads it.
pub(crate) fn hover(app: &mut App, cell: Option<CellLevel>) {
    app.world_mut().insert_resource(DesiredHover(cell));
    app.update();
}

/// The layout [`Display`] of the single entity carrying INSPECT-EXCLUSIVE marker `M` — the
/// GTW-295 discriminating signal for a sub-block's show (`Display::Flex`) / hide
/// (`Display::None`, removed from layout). `None` if the node is missing.
pub(crate) fn display_of<M: Component>(app: &mut App) -> Option<Display> {
    let entity = single_global::<M>(app)?;
    app.world().get::<Node>(entity).map(|n| n.display)
}
