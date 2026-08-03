use bevy::prelude::*;
use gdtf_battle_sim::{
    prelude::{CellLevel, Level},
    visibility::SquadVisibility,
};

use super::seam::PathPreview;

pub(super) const PREVIEW_TINT: Color = Color::srgba(1.0, 0.75, 0.2, 0.5);

/// [`PREVIEW_TINT`]'s alpha — the "remembered" treatment is dimmer than the VISIBLE step
pub(super) const EXPLORED_ALPHA_SCALE: f32 = 0.45;

pub(super) const LINK_MARKER_TINT: Color = Color::srgba(0.3, 0.7, 1.0, 0.7);

pub(super) struct StepDraw {
        pub(super) cell: CellLevel,
    /// The resolved tint — [`PREVIEW_TINT`] (VISIBLE) or its [`EXPLORED_ALPHA_SCALE`]-reduced
        pub(super) tint: Color,
}

///   (VISIBLE → full [`PREVIEW_TINT`]; EXPLORED-not-VISIBLE → [`EXPLORED_ALPHA_SCALE`]-reduced
pub(super) fn preview_draws(
    preview: &PathPreview,
    active_level: Level,
    squad: &SquadVisibility,
) -> Vec<StepDraw> {
    let active_z = i32::from(*active_level);
    let mut draws: Vec<StepDraw> = Vec::new();
    let mut leaves_storey_after: Option<CellLevel> = None;
    let mut seen_off_storey = false;

    for cell in preview.cells() {
        if cell.z == active_z {
            leaves_storey_after = None;
            draws.push(StepDraw {
                cell: *cell,
                tint: step_tint(squad, cell),
            });
        } else {
            seen_off_storey = true;
            if leaves_storey_after.is_none() {
                leaves_storey_after = draws.last().map(|d| d.cell);
            }
        }
    }

    if let Some(link_cell) = leaves_storey_after.filter(|_| seen_off_storey) {
        draws.push(StepDraw {
            cell: link_cell,
            tint: LINK_MARKER_TINT,
        });
    }

    draws
}

/// The §53 tint for a route step at `cell` — full [`PREVIEW_TINT`] when the cell is
fn step_tint(squad: &SquadVisibility, cell: &CellLevel) -> Color {
    if *squad.is_cell_visible(cell) {
        PREVIEW_TINT
    } else {
        PREVIEW_TINT.with_alpha(PREVIEW_TINT.alpha() * EXPLORED_ALPHA_SCALE)
    }
}
