//! Which cardinal sides an emplacement can be entered from.

use bevy_egui::egui;
use gdtf_battle_sim::terrain::facing::TerrainFacing;

use crate::terrain_form::{TerrainDraft, TerrainKindChoice};

const fn side_label(side: TerrainFacing) -> &'static str {
    match side {
        TerrainFacing::North => "North",
        TerrainFacing::East => "East",
        TerrainFacing::South => "South",
        TerrainFacing::West => "West",
    }
}

pub(super) fn entry_sides(ui: &mut egui::Ui, draft: &mut TerrainDraft) {
    if draft.kind() != TerrainKindChoice::Emplacement {
        return;
    }
    ui.label("Entry sides");
    let mut chosen: Vec<TerrainFacing> = draft.entry_sides().to_vec();
    let mut changed = false;
    for side in TerrainFacing::ALL {
        let mut picked = chosen.contains(&side);
        if ui.checkbox(&mut picked, side_label(side)).changed() {
            if picked {
                chosen.push(side);
            } else {
                chosen.retain(|held| *held != side);
            }
            changed = true;
        }
    }
    if changed {
        draft.set_entry_sides(chosen);
    }
}
